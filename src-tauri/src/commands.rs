// =====================================================================
//  commands.rs  -  Comandos Tauri invocables desde Svelte con invoke().
//  Toda la lógica de DB vive aquí (queries en Rust).
// =====================================================================

use crate::db::{self, Db};
use crate::license::{self, LicenseStatus};
use crate::models::*;
use argon2::password_hash::{rand_core::OsRng, PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use rusqlite::{params, OptionalExtension};
use std::sync::Mutex;
use tauri::State;

// =====================================================================
//  SESIÓN Y CONTROL DE ACCESO POR ROL (RBAC)
//  El rol del usuario logueado vive en el backend (no se puede falsear
//  desde el frontend). Los comandos sensibles llaman a exigir_admin().
// =====================================================================

#[derive(Clone)]
pub struct UsuarioSesion {
    pub id_usuario: i64,
    pub usuario: String,
    pub nombre: String,
    pub rol: String,
}

#[derive(Default)]
pub struct Sesion(pub Mutex<Option<UsuarioSesion>>);

/// Falla si no hay un administrador con sesión activa.
fn exigir_admin(sesion: &State<Sesion>) -> Result<(), String> {
    match sesion
        .0
        .lock()
        .map_err(|_| "Error de sesión".to_string())?
        .as_ref()
    {
        Some(u) if u.rol == "Administrador" => Ok(()),
        Some(_) => Err("Solo un administrador puede realizar esta acción.".to_string()),
        None => Err("Debes iniciar sesión como administrador.".to_string()),
    }
}

/// Devuelve el usuario en sesión, o error si nadie ha iniciado sesión.
fn usuario_actual(sesion: &State<Sesion>) -> Result<UsuarioSesion, String> {
    sesion
        .0
        .lock()
        .map_err(|_| "Error de sesión".to_string())?
        .clone()
        .ok_or_else(|| "Debes iniciar sesión.".to_string())
}

/// Inserta un registro en la bitácora. Best-effort: nunca rompe la operación.
fn bitacora(
    pool: &State<Db>,
    sesion: &State<Sesion>,
    accion: &str,
    entidad: &str,
    id_ref: Option<i64>,
    detalle: &str,
) {
    let (id_u, nombre) = match sesion.0.lock().ok().and_then(|s| s.clone()) {
        Some(u) => (Some(u.id_usuario), u.usuario),
        None => (None, "sistema".to_string()),
    };
    if let Ok(conn) = pool.get() {
        let _ = conn.execute(
            "INSERT INTO Bitacora (ID_Usuario, Usuario, Accion, Entidad, ID_Referencia, Detalle)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id_u, nombre, accion, entidad, id_ref, detalle],
        );
    }
}

// =====================================================================
//  LICENCIA
// =====================================================================

/// Días que dura la prueba gratuita antes de pedir licencia.
const DIAS_PRUEBA: i64 = 7;

/// Estado actual de la licencia. Orden de decisión:
///   1) ¿Hay licencia real activada? -> se evalúa su firma/vigencia.
///   2) Si no, ¿sigue dentro del periodo de prueba de 7 días? -> 'Prueba'.
///   3) Si la prueba venció -> 'PruebaExpirada' (pide activar).
#[tauri::command]
pub fn licencia_estado(pool: State<Db>) -> LicenseStatus {
    // 1) Token guardado (licencia real O prueba del servidor cacheada).
    if let Ok(Some(row)) = db::leer_licencia(&pool) {
        if let Some(token) = row.token.filter(|t| !t.is_empty()) {
            return license::evaluate_license(&token, row.ultimo_chequeo);
        }
    }

    // 2) Sin token todavía.
    if license::servidor_configurado() {
        // La prueba la otorga el servidor (anclada por máquina, anti-reset).
        // El frontend debe llamar a `asegurar_trial` para obtenerla.
        return LicenseStatus::invalida(
            "RequiereTrial",
            "Conéctate a internet para iniciar tu prueba.",
        );
    }

    // 3) Modo desarrollo (sin servidor): prueba local por Configuracion.
    trial_local(&pool)
}

/// Prueba local (solo cuando el servidor no está configurado). Se resetea si
/// borran la DB; por eso en producción se usa la prueba anclada al servidor.
fn trial_local(pool: &Db) -> LicenseStatus {
    let inicio = match db::trial_inicio(pool) {
        Ok(d) => d,
        Err(e) => return LicenseStatus::invalida("Error", &e),
    };
    let ahora = chrono::Utc::now();
    let expira = inicio + chrono::Duration::days(DIAS_PRUEBA);
    let dias = (((expira - ahora).num_seconds() as f64) / 86_400.0).ceil() as i64;

    if ahora < expira {
        LicenseStatus {
            valida: true,
            estado: "Prueba".into(),
            mensaje: "Versión de prueba (local).".into(),
            fecha_expiracion: Some(expira),
            dias_restantes: Some(dias.max(0)),
        }
    } else {
        LicenseStatus {
            valida: false,
            estado: "PruebaExpirada".into(),
            mensaje: "Terminó tu prueba gratis. Activa una licencia para seguir usando el POS."
                .into(),
            fecha_expiracion: Some(expira),
            dias_restantes: Some(0),
        }
    }
}

/// Obtiene o refresca la prueba. Si hay token válido lo usa; si no, y el
/// servidor está configurado, pide al servidor el token de prueba (anclado por
/// máquina) y lo cachea. Sin servidor configurado, cae a la prueba local.
#[tauri::command]
pub async fn asegurar_trial(pool: State<'_, Db>) -> Result<LicenseStatus, String> {
    let pool = pool.inner().clone();

    if let Ok(Some(row)) = db::leer_licencia(&pool) {
        if let Some(token) = row.token.filter(|t| !t.is_empty()) {
            let st = license::evaluate_license(&token, row.ultimo_chequeo);
            if st.valida {
                return Ok(st);
            }
        }
    }

    if !license::servidor_configurado() {
        return Ok(trial_local(&pool));
    }

    let token = license::solicitar_trial().await?;
    let payload = license::verify_token_signature(&token)?;
    db::guardar_licencia(&pool, "TRIAL", &token, &payload)?;
    Ok(license::evaluate_license(&token, Some(chrono::Utc::now())))
}

/// Activa una clave: pide el token firmado al servidor, lo verifica y lo
/// guarda en LicenciaLocal. Devuelve el estado resultante.
#[tauri::command]
pub async fn licencia_activar(pool: State<'_, Db>, clave: String) -> Result<LicenseStatus, String> {
    let pool = pool.inner().clone(); // el Pool es Arc; barato y evita líos de lifetime con await

    let token = license::solicitar_token(&clave).await?;
    let payload = license::verify_token_signature(&token)?;
    db::guardar_licencia(&pool, &clave, &token, &payload)?;

    Ok(license::evaluate_license(&token, Some(chrono::Utc::now())))
}

/// Revalida en línea con la clave ya guardada (llamar en segundo plano si
/// hay internet). Refresca el token y UltimoChequeoOnline.
#[tauri::command]
pub async fn licencia_revalidar(pool: State<'_, Db>) -> Result<LicenseStatus, String> {
    let pool = pool.inner().clone();

    let clave = db::leer_licencia(&pool)?
        .and_then(|r| r.clave)
        .ok_or("No hay licencia activada para revalidar")?;

    let token = license::solicitar_token(&clave).await?;
    let payload = license::verify_token_signature(&token)?;
    db::guardar_licencia(&pool, &clave, &token, &payload)?;

    Ok(license::evaluate_license(&token, Some(chrono::Utc::now())))
}

// =====================================================================
//  CATÁLOGO
// =====================================================================

#[tauri::command]
pub fn listar_categorias(
    pool: State<Db>,
    incluir_inactivos: Option<bool>,
) -> Result<Vec<Categoria>, String> {
    let todos = incluir_inactivos.unwrap_or(false) as i64;
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT ID_Categoria, Categoria, Descripcion, Activo
             FROM Categorias WHERE (?1 = 1 OR Activo = 1) ORDER BY Categoria",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([todos], |r| {
            Ok(Categoria {
                id_categoria: r.get(0)?,
                categoria: r.get(1)?,
                descripcion: r.get(2)?,
                activo: r.get::<_, i64>(3)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn actualizar_categoria(
    pool: State<Db>,
    sesion: State<Sesion>,
    id: i64,
    categoria: String,
    descripcion: Option<String>,
    activo: bool,
) -> Result<(), String> {
    exigir_admin(&sesion)?;
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE Categorias SET Categoria = ?1, Descripcion = ?2, Activo = ?3 WHERE ID_Categoria = ?4",
        params![categoria, descripcion, activo as i64, id],
    )
    .map_err(|e| e.to_string())?;
    bitacora(&pool, &sesion, "Edición", "Categoría", Some(id), &categoria);
    Ok(())
}

#[tauri::command]
pub fn listar_unidades(pool: State<Db>) -> Result<Vec<UnidadMedida>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT ID_UnidadMedida, UnidadMedida FROM UnidadMedida ORDER BY ID_UnidadMedida")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(UnidadMedida {
                id_unidad_medida: r.get(0)?,
                unidad_medida: r.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn crear_categoria(
    pool: State<Db>,
    sesion: State<Sesion>,
    categoria: String,
    descripcion: Option<String>,
) -> Result<i64, String> {
    exigir_admin(&sesion)?;
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO Categorias (Categoria, Descripcion) VALUES (?1, ?2)",
        params![categoria, descripcion],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    bitacora(&pool, &sesion, "Alta", "Categoría", Some(id), &categoria);
    Ok(id)
}

#[tauri::command]
pub fn crear_unidad(pool: State<Db>, sesion: State<Sesion>, nombre: String) -> Result<i64, String> {
    exigir_admin(&sesion)?;
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO UnidadMedida (UnidadMedida) VALUES (?1)",
        params![nombre],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn actualizar_unidad(
    pool: State<Db>,
    sesion: State<Sesion>,
    id: i64,
    nombre: String,
) -> Result<(), String> {
    exigir_admin(&sesion)?;
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE UnidadMedida SET UnidadMedida = ?1 WHERE ID_UnidadMedida = ?2",
        params![nombre, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn listar_metodos_pago(
    pool: State<Db>,
    incluir_inactivos: Option<bool>,
) -> Result<Vec<MetodoPago>, String> {
    let todos = incluir_inactivos.unwrap_or(false) as i64;
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT ID_MetodoPago, MetodoPago, RequiereReferencia, Activo
             FROM MetodosPago WHERE (?1 = 1 OR Activo = 1) ORDER BY ID_MetodoPago",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([todos], |r| {
            Ok(MetodoPago {
                id_metodo_pago: r.get(0)?,
                metodo_pago: r.get(1)?,
                requiere_referencia: r.get::<_, i64>(2)? != 0,
                activo: r.get::<_, i64>(3)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn crear_metodo_pago(
    pool: State<Db>,
    sesion: State<Sesion>,
    metodo_pago: String,
    requiere_referencia: bool,
) -> Result<i64, String> {
    exigir_admin(&sesion)?;
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO MetodosPago (MetodoPago, RequiereReferencia) VALUES (?1, ?2)",
        params![metodo_pago, requiere_referencia as i64],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Ya existe un método de pago con ese nombre".to_string()
        } else {
            e.to_string()
        }
    })?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn actualizar_metodo_pago(
    pool: State<Db>,
    sesion: State<Sesion>,
    id: i64,
    metodo_pago: String,
    requiere_referencia: bool,
    activo: bool,
) -> Result<(), String> {
    exigir_admin(&sesion)?;
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE MetodosPago SET MetodoPago = ?1, RequiereReferencia = ?2, Activo = ?3
         WHERE ID_MetodoPago = ?4",
        params![metodo_pago, requiere_referencia as i64, activo as i64, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ---- Monedas y denominaciones ----

#[tauri::command]
pub fn listar_monedas(
    pool: State<Db>,
    incluir_inactivos: Option<bool>,
) -> Result<Vec<Moneda>, String> {
    let todos = incluir_inactivos.unwrap_or(false) as i64;
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT ID_Moneda, Moneda, Codigo, Simbolo, EsPrincipal, Activo
             FROM Monedas WHERE (?1 = 1 OR Activo = 1) ORDER BY EsPrincipal DESC, Moneda",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([todos], |r| {
            Ok(Moneda {
                id_moneda: r.get(0)?,
                moneda: r.get(1)?,
                codigo: r.get(2)?,
                simbolo: r.get(3)?,
                es_principal: r.get::<_, i64>(4)? != 0,
                activo: r.get::<_, i64>(5)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn crear_moneda(
    pool: State<Db>,
    sesion: State<Sesion>,
    moneda: String,
    codigo: String,
    simbolo: Option<String>,
    es_principal: bool,
) -> Result<i64, String> {
    exigir_admin(&sesion)?;
    let conn = pool.get().map_err(|e| e.to_string())?;
    if es_principal {
        conn.execute("UPDATE Monedas SET EsPrincipal = 0", [])
            .map_err(|e| e.to_string())?;
    }
    conn.execute(
        "INSERT INTO Monedas (Moneda, Codigo, Simbolo, EsPrincipal) VALUES (?1, ?2, ?3, ?4)",
        params![moneda, codigo, simbolo, es_principal as i64],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Ya existe una moneda con ese código".to_string()
        } else {
            e.to_string()
        }
    })?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn actualizar_moneda(
    pool: State<Db>,
    sesion: State<Sesion>,
    id: i64,
    moneda: String,
    codigo: String,
    simbolo: Option<String>,
    es_principal: bool,
    activo: bool,
) -> Result<(), String> {
    exigir_admin(&sesion)?;
    let conn = pool.get().map_err(|e| e.to_string())?;
    if es_principal {
        conn.execute("UPDATE Monedas SET EsPrincipal = 0", [])
            .map_err(|e| e.to_string())?;
    }
    conn.execute(
        "UPDATE Monedas SET Moneda = ?1, Codigo = ?2, Simbolo = ?3, EsPrincipal = ?4, Activo = ?5
         WHERE ID_Moneda = ?6",
        params![moneda, codigo, simbolo, es_principal as i64, activo as i64, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn listar_denominaciones(
    pool: State<Db>,
    id_moneda: i64,
) -> Result<Vec<Denominacion>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT ID_Denominacion, ID_Moneda, Valor, Tipo, Activo
             FROM Denominaciones WHERE ID_Moneda = ?1 AND Activo = 1 ORDER BY Valor DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([id_moneda], |r| {
            Ok(Denominacion {
                id_denominacion: r.get(0)?,
                id_moneda: r.get(1)?,
                valor: r.get(2)?,
                tipo: r.get(3)?,
                activo: r.get::<_, i64>(4)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn crear_denominacion(
    pool: State<Db>,
    sesion: State<Sesion>,
    id_moneda: i64,
    valor: f64,
    tipo: Option<String>,
) -> Result<i64, String> {
    exigir_admin(&sesion)?;
    if valor <= 0.0 {
        return Err("El valor debe ser mayor a cero".into());
    }
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO Denominaciones (ID_Moneda, Valor, Tipo) VALUES (?1, ?2, ?3)",
        params![id_moneda, valor, tipo],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn eliminar_denominacion(pool: State<Db>, sesion: State<Sesion>, id: i64) -> Result<(), String> {
    exigir_admin(&sesion)?;
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM Denominaciones WHERE ID_Denominacion = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---- Configuración (datos del negocio para el ticket) ----

#[tauri::command]
pub fn listar_config(pool: State<Db>) -> Result<Vec<ConfigItem>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT Clave, COALESCE(Valor, '') FROM Configuracion
             WHERE Clave <> 'trial_inicio' ORDER BY Clave",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ConfigItem {
                clave: r.get(0)?,
                valor: r.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn guardar_config(
    pool: State<Db>,
    sesion: State<Sesion>,
    items: Vec<ConfigItem>,
) -> Result<(), String> {
    exigir_admin(&sesion)?;
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for it in &items {
        if it.clave == "trial_inicio" {
            continue; // llave interna, no se toca
        }
        tx.execute(
            "INSERT INTO Configuracion (Clave, Valor) VALUES (?1, ?2)
             ON CONFLICT(Clave) DO UPDATE SET Valor = excluded.Valor",
            params![it.clave, it.valor],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    bitacora(&pool, &sesion, "Edición", "Configuración", None, "Datos del negocio");
    Ok(())
}

#[tauri::command]
pub fn listar_productos(
    pool: State<Db>,
    incluir_inactivos: Option<bool>,
) -> Result<Vec<Producto>, String> {
    let todos = incluir_inactivos.unwrap_or(false) as i64;
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT p.ID_Producto, p.Producto, p.CodigoBarras, p.PrecioUnitario, p.PrecioCosto,
                    p.SeVendePeso, p.StockMinimo, p.ID_Categoria, c.Categoria,
                    CASE WHEN p.ManejaCaducidad = 1
                     THEN COALESCE((SELECT SUM(l.Cantidad) FROM Lotes l WHERE l.ID_Producto = p.ID_Producto), 0)
                     ELSE COALESCE(i.Cantidad, 0) END,
                p.Activo, p.ID_UnidadMedida, p.Tipo, p.ManejaCaducidad, um.UnidadMedida
             FROM Productos p
             LEFT JOIN Categorias c ON c.ID_Categoria = p.ID_Categoria
             LEFT JOIN UnidadMedida um ON um.ID_UnidadMedida = p.ID_UnidadMedida
             LEFT JOIN Inventario i ON i.ID_Producto = p.ID_Producto
             WHERE (?1 = 1 OR p.Activo = 1)
             ORDER BY p.Producto",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([todos], fila_a_producto)
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// Para el lector de código de barras: devuelve el producto o None.
#[tauri::command]
pub fn buscar_producto_por_codigo(
    pool: State<Db>,
    codigo: String,
) -> Result<Option<Producto>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT p.ID_Producto, p.Producto, p.CodigoBarras, p.PrecioUnitario, p.PrecioCosto,
                p.SeVendePeso, p.StockMinimo, p.ID_Categoria, c.Categoria,
                CASE WHEN p.ManejaCaducidad = 1
                     THEN COALESCE((SELECT SUM(l.Cantidad) FROM Lotes l WHERE l.ID_Producto = p.ID_Producto), 0)
                     ELSE COALESCE(i.Cantidad, 0) END,
                p.Activo, p.ID_UnidadMedida, p.Tipo, p.ManejaCaducidad, um.UnidadMedida
         FROM Productos p
         LEFT JOIN Categorias c ON c.ID_Categoria = p.ID_Categoria
         LEFT JOIN UnidadMedida um ON um.ID_UnidadMedida = p.ID_UnidadMedida
         LEFT JOIN Inventario i ON i.ID_Producto = p.ID_Producto
         WHERE p.CodigoBarras = ?1 AND p.Activo = 1",
        [codigo],
        fila_a_producto,
    )
    .optional()
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn crear_producto(
    pool: State<Db>,
    sesion: State<Sesion>,
    datos: NuevoProducto,
) -> Result<i64, String> {
    exigir_admin(&sesion)?;
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let es_servicio = datos.tipo.eq_ignore_ascii_case("Servicio");
    let maneja_caducidad = !es_servicio && datos.maneja_caducidad;

    tx.execute(
        "INSERT INTO Productos
            (Producto, CodigoBarras, PrecioUnitario, PrecioCosto, Tipo, ManejaCaducidad,
             SeVendePeso, StockMinimo, ID_UnidadMedida, ID_Categoria)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            datos.producto,
            datos.codigo_barras,
            datos.precio_unitario,
            datos.precio_costo,
            if es_servicio { "Servicio" } else { "Producto" },
            maneja_caducidad as i64,
            datos.se_vende_peso as i64,
            datos.stock_minimo,
            datos.id_unidad_medida,
            datos.id_categoria,
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = tx.last_insert_rowid();

    // Los servicios no manejan inventario (no descuentan stock al venderse).
    if !es_servicio {
        if maneja_caducidad {
            // El stock vive en Lotes; crea el lote inicial si hay existencia.
            if datos.existencia_inicial != 0.0 {
                agregar_lote(
                    &tx,
                    id,
                    datos.lote_inicial.as_deref(),
                    datos.caducidad_inicial.as_deref(),
                    datos.existencia_inicial,
                )?;
                tx.execute(
                    "INSERT INTO MovimientosInventario (ID_Producto, Tipo, Cantidad, Motivo, CostoUnitario)
                     VALUES (?1, 'Entrada', ?2, 'Alta de producto', ?3)",
                    params![id, datos.existencia_inicial, datos.precio_costo],
                )
                .map_err(|e| e.to_string())?;
            }
        } else {
            // Fila de inventario con la existencia inicial
            tx.execute(
                "INSERT INTO Inventario (ID_Producto, Cantidad) VALUES (?1, ?2)",
                params![id, datos.existencia_inicial],
            )
            .map_err(|e| e.to_string())?;

            if datos.existencia_inicial != 0.0 {
                tx.execute(
                    "INSERT INTO MovimientosInventario (ID_Producto, Tipo, Cantidad, Motivo, CostoUnitario)
                     VALUES (?1, 'Entrada', ?2, 'Alta de producto', ?3)",
                    params![id, datos.existencia_inicial, datos.precio_costo],
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    // Capa de costo PEPS de la existencia inicial (al precio de costo capturado).
    if !es_servicio && datos.existencia_inicial > 0.0 {
        agregar_capa(&tx, id, datos.existencia_inicial, datos.precio_costo)?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    bitacora(&pool, &sesion, "Alta", "Producto", Some(id), &datos.producto);
    Ok(id)
}

// =====================================================================
//  VENTAS  (transacción: ticket + detalle + inventario + kardex)
// =====================================================================

#[tauri::command]
pub fn registrar_venta(
    pool: State<Db>,
    sesion: State<Sesion>,
    venta: NuevaVenta,
) -> Result<VentaResultado, String> {
    usuario_actual(&sesion)?;
    if venta.items.is_empty() {
        return Err("La venta no tiene productos".into());
    }

    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // Exigir caja abierta: la venta se liga al corte abierto del usuario
    // (no se confía en el ID que mande el front).
    let id_corte: i64 = tx
        .query_row(
            "SELECT ID_Corte FROM CortesCaja WHERE ID_Usuario = ?1 AND Estatus = 'Abierto'
             ORDER BY ID_Corte DESC LIMIT 1",
            [venta.id_usuario],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or("Debes abrir la caja (corte) antes de registrar ventas.")?;

    // 1) Calcular el total con el precio ACTUAL del servidor (no se confía
    //    en un precio enviado por el front).
    let mut total = 0.0_f64;
    // (id_producto, cantidad, precio, es_servicio, maneja_caducidad)
    let mut lineas: Vec<(i64, f64, f64, bool, bool)> = Vec::with_capacity(venta.items.len());
    for item in &venta.items {
        if item.cantidad <= 0.0 {
            return Err(format!("Cantidad inválida para el producto {}", item.id_producto));
        }
        let (precio, tipo, maneja): (f64, String, i64) = tx
            .query_row(
                "SELECT PrecioUnitario, Tipo, ManejaCaducidad
                 FROM Productos WHERE ID_Producto = ?1 AND Activo = 1",
                [item.id_producto],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|_| format!("Producto {} no encontrado o inactivo", item.id_producto))?;

        total += precio * item.cantidad;
        lineas.push((
            item.id_producto,
            item.cantidad,
            precio,
            tipo.eq_ignore_ascii_case("Servicio"),
            maneja != 0,
        ));
    }

    let cambio = venta.pago_con.map(|p| p - total);

    // 2) Cabecera (Folio se asigna después con el ID generado)
    tx.execute(
        "INSERT INTO Ventas
            (Folio, ID_Usuario, ID_MetodoPago, ID_Corte, ID_Cliente, ReferenciaPago,
             Total, PagoCon, Cambio)
         VALUES (NULL, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            venta.id_usuario,
            venta.id_metodo_pago,
            id_corte,
            venta.id_cliente,
            venta.referencia_pago,
            total,
            venta.pago_con,
            cambio,
        ],
    )
    .map_err(|e| e.to_string())?;

    let id_venta = tx.last_insert_rowid();
    let folio = format!("T{:06}", id_venta);
    tx.execute(
        "UPDATE Ventas SET Folio = ?1 WHERE ID_Venta = ?2",
        params![folio, id_venta],
    )
    .map_err(|e| e.to_string())?;

    // 3) Detalle + descuento de inventario + kardex
    for (id_producto, cantidad, precio, es_servicio, maneja_cad) in lineas {
        // Costo de venta PEPS de esta línea (consume las capas más antiguas).
        // Servicios = 0 (no manejan inventario ni costo).
        let costo_linea = if es_servicio {
            0.0
        } else {
            consumir_capas_fifo(&tx, id_producto, cantidad)?
        };
        tx.execute(
            "INSERT INTO Detalle_Ventas
                (ID_Venta, ID_Producto, Cantidad, PrecioVentaHistorico, CostoHistorico)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id_venta, id_producto, cantidad, precio, costo_linea],
        )
        .map_err(|e| e.to_string())?;

        // Los servicios no afectan inventario.
        if es_servicio {
            continue;
        }

        if maneja_cad {
            // El stock vive en lotes: se descuenta FEFO (caduca antes = sale antes).
            consumir_lotes_fefo(&tx, id_producto, cantidad)?;
        } else {
            tx.execute(
                "INSERT INTO Inventario (ID_Producto, Cantidad) VALUES (?1, 0)
                 ON CONFLICT(ID_Producto) DO NOTHING",
                params![id_producto],
            )
            .map_err(|e| e.to_string())?;

            tx.execute(
                "UPDATE Inventario
                 SET Cantidad = Cantidad - ?1, FechaModificacion = CURRENT_TIMESTAMP
                 WHERE ID_Producto = ?2",
                params![cantidad, id_producto],
            )
            .map_err(|e| e.to_string())?;
        }

        tx.execute(
            "INSERT INTO MovimientosInventario (ID_Producto, Tipo, Cantidad, Motivo, ID_Usuario, CostoUnitario)
             VALUES (?1, 'Venta', ?2, 'Venta', ?3, ?4)",
            params![
                id_producto,
                -cantidad,
                venta.id_usuario,
                if cantidad > 0.0 { costo_linea / cantidad } else { 0.0 }
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    // Fiado: si la venta es a crédito (método 'Fiado') y trae cliente,
    // se le carga el total a su saldo.
    if let Some(id_cliente) = venta.id_cliente {
        let metodo: String = tx
            .query_row(
                "SELECT MetodoPago FROM MetodosPago WHERE ID_MetodoPago = ?1",
                [venta.id_metodo_pago],
                |r| r.get(0),
            )
            .unwrap_or_default();
        if metodo.eq_ignore_ascii_case("Fiado") {
            tx.execute(
                "UPDATE Clientes SET SaldoFiado = SaldoFiado + ?1 WHERE ID_Cliente = ?2",
                params![total, id_cliente],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    bitacora(
        &pool,
        &sesion,
        "Venta",
        "Venta",
        Some(id_venta),
        &format!("{folio} · {total:.2}"),
    );

    Ok(VentaResultado {
        id_venta,
        folio,
        total,
        cambio,
    })
}

/// Cancela una venta: la marca 'Cancelada', aplica la disposición elegida a la
/// mercancía y, si fue fiado, revierte el saldo del cliente.
///
/// `disposicion` (solo afecta productos; los servicios nunca tocan inventario):
///   - "regresar":  la mercancía vuelve al inventario (Entrada).
///   - "merma":     no vuelve al stock; se registra como Merma para reportes.
///   - "descartar": no vuelve al stock ni se registra como merma.
#[tauri::command]
pub fn cancelar_venta(
    pool: State<Db>,
    sesion: State<Sesion>,
    id_venta: i64,
    id_usuario: i64,
    disposicion: String,
) -> Result<(), String> {
    exigir_admin(&sesion)?;
    let disp = match disposicion.to_lowercase().as_str() {
        d @ ("regresar" | "merma" | "descartar") => d.to_string(),
        _ => return Err("Disposición inválida".into()),
    };
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let (estatus, total, id_cliente, id_metodo): (String, f64, Option<i64>, i64) = tx
        .query_row(
            "SELECT Estatus, Total, ID_Cliente, ID_MetodoPago FROM Ventas WHERE ID_Venta = ?1",
            [id_venta],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|_| "Venta no encontrada".to_string())?;

    if estatus != "Completada" {
        return Err("La venta ya estaba cancelada.".into());
    }

    // Disposición de cada producto (los servicios no manejan stock).
    let lineas: Vec<(i64, f64, String, i64, f64)> = {
        let mut stmt = tx
            .prepare(
                "SELECT dv.ID_Producto, dv.Cantidad, COALESCE(p.Tipo, 'Producto'),
                        COALESCE(p.ManejaCaducidad, 0), COALESCE(dv.CostoHistorico, 0)
                 FROM Detalle_Ventas dv
                 LEFT JOIN Productos p ON p.ID_Producto = dv.ID_Producto
                 WHERE dv.ID_Venta = ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([id_venta], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())?
    };

    for (id_prod, cant, tipo, maneja, costo_hist) in &lineas {
        if tipo.eq_ignore_ascii_case("Servicio") {
            continue; // los servicios no afectan inventario
        }
        let maneja_cad = *maneja != 0;
        match disp.as_str() {
            "regresar" if maneja_cad => {
                // Reingresa a un lote (sin caducidad; el admin la ajusta si aplica).
                agregar_lote(&tx, *id_prod, None, None, *cant)?;
                agregar_capa(&tx, *id_prod, *cant, if *cant > 0.0 { *costo_hist / *cant } else { 0.0 })?;
                tx.execute(
                    "INSERT INTO MovimientosInventario (ID_Producto, Tipo, Cantidad, Motivo, ID_Usuario, CostoUnitario)
                     VALUES (?1, 'Entrada', ?2, 'Devolución por cancelación', ?3, ?4)",
                    params![id_prod, cant, id_usuario, if *cant > 0.0 { *costo_hist / *cant } else { 0.0 }],
                )
                .map_err(|e| e.to_string())?;
            }
            "regresar" => {
                tx.execute(
                    "INSERT INTO Inventario (ID_Producto, Cantidad) VALUES (?1, 0)
                     ON CONFLICT(ID_Producto) DO NOTHING",
                    params![id_prod],
                )
                .map_err(|e| e.to_string())?;
                tx.execute(
                    "UPDATE Inventario SET Cantidad = Cantidad + ?1, FechaModificacion = CURRENT_TIMESTAMP
                     WHERE ID_Producto = ?2",
                    params![cant, id_prod],
                )
                .map_err(|e| e.to_string())?;
                agregar_capa(&tx, *id_prod, *cant, if *cant > 0.0 { *costo_hist / *cant } else { 0.0 })?;
                tx.execute(
                    "INSERT INTO MovimientosInventario (ID_Producto, Tipo, Cantidad, Motivo, ID_Usuario, CostoUnitario)
                     VALUES (?1, 'Entrada', ?2, 'Devolución por cancelación', ?3, ?4)",
                    params![id_prod, cant, id_usuario, if *cant > 0.0 { *costo_hist / *cant } else { 0.0 }],
                )
                .map_err(|e| e.to_string())?;
            }
            "merma" => {
                // No vuelve al stock (ya estaba descontado por la venta). Se
                // documenta con dos asientos que se netean: reverso de la venta
                // y baja por merma. El stock queda igual (descontado).
                tx.execute(
                    "INSERT INTO MovimientosInventario (ID_Producto, Tipo, Cantidad, Motivo, ID_Usuario, CostoUnitario)
                     VALUES (?1, 'Entrada', ?2, 'Reverso por cancelación', ?3, ?4)",
                    params![id_prod, cant, id_usuario, if *cant > 0.0 { *costo_hist / *cant } else { 0.0 }],
                )
                .map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT INTO MovimientosInventario (ID_Producto, Tipo, Cantidad, Motivo, ID_Usuario, CostoUnitario)
                     VALUES (?1, 'Merma', ?2, 'Merma por cancelación', ?3, ?4)",
                    params![id_prod, -cant, id_usuario, if *cant > 0.0 { *costo_hist / *cant } else { 0.0 }],
                )
                .map_err(|e| e.to_string())?;
            }
            // "descartar": no se regresa ni se registra merma; el stock queda
            // descontado tal cual quedó con la venta.
            _ => {}
        }
    }

    // Revertir fiado si aplica
    if let Some(cli) = id_cliente {
        let metodo: String = tx
            .query_row(
                "SELECT MetodoPago FROM MetodosPago WHERE ID_MetodoPago = ?1",
                [id_metodo],
                |r| r.get(0),
            )
            .unwrap_or_default();
        if metodo.eq_ignore_ascii_case("Fiado") {
            tx.execute(
                "UPDATE Clientes SET SaldoFiado = SaldoFiado - ?1 WHERE ID_Cliente = ?2",
                params![total, cli],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    tx.execute(
        "UPDATE Ventas SET Estatus = 'Cancelada' WHERE ID_Venta = ?1",
        [id_venta],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    bitacora(
        &pool,
        &sesion,
        "Cancelación",
        "Venta",
        Some(id_venta),
        &format!("Cancelada · {total:.2} · mercancía: {disp}"),
    );
    Ok(())
}

/// Historial de ventas entre dos fechas (formato 'YYYY-MM-DD', inclusivo).
#[tauri::command]
pub fn listar_ventas(
    pool: State<Db>,
    desde: String,
    hasta: String,
) -> Result<Vec<VentaResumen>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT v.ID_Venta, v.Folio, v.FechaVenta, v.Total, v.Estatus,
                    m.MetodoPago, u.Nombre, c.Nombre
             FROM Ventas v
             JOIN MetodosPago m ON m.ID_MetodoPago = v.ID_MetodoPago
             JOIN Usuarios u ON u.ID_Usuario = v.ID_Usuario
             LEFT JOIN Clientes c ON c.ID_Cliente = v.ID_Cliente
             WHERE date(v.FechaVenta, 'localtime') BETWEEN ?1 AND ?2
             ORDER BY v.ID_Venta DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![desde, hasta], |r| {
            Ok(VentaResumen {
                id_venta: r.get(0)?,
                folio: r.get(1)?,
                fecha_venta: r.get(2)?,
                total: r.get(3)?,
                estatus: r.get(4)?,
                metodo_pago: r.get(5)?,
                usuario: r.get(6)?,
                cliente: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn detalle_venta(pool: State<Db>, id_venta: i64) -> Result<Vec<DetalleVentaLinea>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT p.Producto, d.Cantidad, d.PrecioVentaHistorico
             FROM Detalle_Ventas d
             JOIN Productos p ON p.ID_Producto = d.ID_Producto
             WHERE d.ID_Venta = ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([id_venta], |r| {
            Ok(DetalleVentaLinea {
                producto: r.get(0)?,
                cantidad: r.get(1)?,
                precio: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// Cierre de ventas: totales y desglose por método (solo ventas completadas).
#[tauri::command]
pub fn reporte_ventas(
    pool: State<Db>,
    desde: String,
    hasta: String,
) -> Result<ReporteVentas, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;

    let metodos: Vec<ReporteMetodo> = {
        let mut stmt = conn
            .prepare(
                "SELECT m.MetodoPago, COALESCE(SUM(v.Total), 0), COUNT(*)
                 FROM Ventas v
                 JOIN MetodosPago m ON m.ID_MetodoPago = v.ID_MetodoPago
                 WHERE v.Estatus = 'Completada' AND date(v.FechaVenta, 'localtime') BETWEEN ?1 AND ?2
                 GROUP BY m.MetodoPago
                 ORDER BY 2 DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![desde, hasta], |r| {
                Ok(ReporteMetodo {
                    metodo_pago: r.get(0)?,
                    total: r.get(1)?,
                    tickets: r.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())?
    };

    let total: f64 = metodos.iter().map(|m| m.total).sum();
    let tickets: i64 = metodos.iter().map(|m| m.tickets).sum();

    let canceladas: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM Ventas
             WHERE Estatus = 'Cancelada' AND date(FechaVenta, 'localtime') BETWEEN ?1 AND ?2",
            params![desde, hasta],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(ReporteVentas {
        total,
        tickets,
        canceladas,
        metodos,
    })
}

// =====================================================================
//  IMPORTACIÓN DE CATÁLOGOS (validada, normalizada y parametrizada)
// =====================================================================

/// Normaliza texto: recorta espacios (incluye dobles internos) y MAYÚSCULAS.
fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_uppercase()
}

#[tauri::command]
pub fn importar_productos(
    pool: State<Db>,
    sesion: State<Sesion>,
    filas: Vec<FilaProductoImport>,
) -> Result<ResultadoImport, String> {
    exigir_admin(&sesion)?;
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut insertados = 0i64;
    let mut omitidos = 0i64;
    let mut errores: Vec<String> = Vec::new();

    for (i, f) in filas.iter().enumerate() {
        let renglon = i + 2; // +1 por encabezado, +1 por base-1
        let nombre = norm(&f.producto);
        if nombre.is_empty() {
            omitidos += 1;
            errores.push(format!("Fila {renglon}: nombre vacío"));
            continue;
        }
        if f.precio_venta < 0.0 || f.precio_costo.unwrap_or(0.0) < 0.0 {
            omitidos += 1;
            errores.push(format!("Fila {renglon}: precio negativo"));
            continue;
        }

        let codigo: Option<String> = f
            .codigo_barras
            .as_ref()
            .map(|c| norm(c))
            .filter(|c| !c.is_empty());

        // Categoría por nombre (se crea si no existe)
        let id_cat: Option<i64> = match &f.categoria {
            Some(c) if !c.trim().is_empty() => {
                let cn = norm(c);
                let found: Option<i64> = tx
                    .query_row(
                        "SELECT ID_Categoria FROM Categorias WHERE UPPER(Categoria) = ?1",
                        [&cn],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(|e| e.to_string())?;
                match found {
                    Some(id) => Some(id),
                    None => {
                        tx.execute("INSERT INTO Categorias (Categoria) VALUES (?1)", [&cn])
                            .map_err(|e| e.to_string())?;
                        Some(tx.last_insert_rowid())
                    }
                }
            }
            _ => None,
        };

        // Unidad de medida por nombre; si no viene en el archivo, "Pieza".
        // Así nunca queda en NULL. Se crea la unidad si no existe (como categoría).
        let unidad_nombre = f
            .unidad
            .as_ref()
            .map(|u| norm(u))
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| norm("Pieza"));
        let id_uni: i64 = {
            let found: Option<i64> = tx
                .query_row(
                    "SELECT ID_UnidadMedida FROM UnidadMedida WHERE UPPER(UnidadMedida) = ?1",
                    [&unidad_nombre],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            match found {
                Some(id) => id,
                None => {
                    tx.execute(
                        "INSERT INTO UnidadMedida (UnidadMedida) VALUES (?1)",
                        [&unidad_nombre],
                    )
                    .map_err(|e| e.to_string())?;
                    tx.last_insert_rowid()
                }
            }
        };

        let res = tx.execute(
            "INSERT INTO Productos
                (Producto, CodigoBarras, PrecioUnitario, PrecioCosto, ID_Categoria, ID_UnidadMedida, SeVendePeso)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                nombre,
                codigo,
                f.precio_venta,
                f.precio_costo.unwrap_or(0.0),
                id_cat,
                id_uni,
                f.se_vende_peso.unwrap_or(false) as i64
            ],
        );

        match res {
            Ok(_) => {
                let id = tx.last_insert_rowid();
                // Catálogo: el producto nace con 0 existencia. El stock entra por
                // Compras. NO se crea capa PEPS aquí para no chocar con el costeo
                // ni duplicar inventario (ideal para dar de alta productos nuevos).
                tx.execute(
                    "INSERT INTO Inventario (ID_Producto, Cantidad) VALUES (?1, 0)",
                    [id],
                )
                .map_err(|e| e.to_string())?;
                insertados += 1;
            }
            Err(e) => {
                omitidos += 1;
                let msg = if e.to_string().contains("UNIQUE") {
                    "código de barras duplicado".to_string()
                } else {
                    e.to_string()
                };
                errores.push(format!("Fila {renglon} ({nombre}): {msg}"));
            }
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    bitacora(
        &pool,
        &sesion,
        "Importación",
        "Producto",
        None,
        &format!("{insertados} productos importados, {omitidos} omitidos"),
    );
    Ok(ResultadoImport {
        insertados,
        omitidos,
        errores,
    })
}

/// Carga inicial de inventario ("al inaugurar"). A diferencia de `importar_productos`,
/// busca el producto por código o nombre, y si ya existe lo REUTILIZA. En todos los
/// casos RESETEA el stock de cada producto del archivo (borra sus capas, lotes y
/// movimientos de "Inventario inicial") y lo vuelve a sembrar con la existencia y el
/// costo del CSV. Es idempotente: re-correr el mismo archivo deja el mismo resultado.
/// NO toca productos que no estén en el archivo.
#[tauri::command]
pub fn iniciar_inventario(
    pool: State<Db>,
    sesion: State<Sesion>,
    filas: Vec<FilaProductoImport>,
) -> Result<ResultadoImport, String> {
    exigir_admin(&sesion)?;
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut insertados = 0i64;
    let mut omitidos = 0i64;
    let mut errores: Vec<String> = Vec::new();

    for (i, f) in filas.iter().enumerate() {
        let renglon = i + 2;
        let nombre = norm(&f.producto);
        if nombre.is_empty() {
            omitidos += 1;
            errores.push(format!("Fila {renglon}: nombre vacío"));
            continue;
        }
        if f.precio_venta < 0.0 || f.precio_costo.unwrap_or(0.0) < 0.0 {
            omitidos += 1;
            errores.push(format!("Fila {renglon}: precio negativo"));
            continue;
        }

        let codigo: Option<String> = f
            .codigo_barras
            .as_ref()
            .map(|c| norm(c))
            .filter(|c| !c.is_empty());

        // Categoría por nombre (se crea si no existe)
        let id_cat: Option<i64> = match &f.categoria {
            Some(c) if !c.trim().is_empty() => {
                let cn = norm(c);
                let found: Option<i64> = tx
                    .query_row(
                        "SELECT ID_Categoria FROM Categorias WHERE UPPER(Categoria) = ?1",
                        [&cn],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(|e| e.to_string())?;
                match found {
                    Some(id) => Some(id),
                    None => {
                        tx.execute("INSERT INTO Categorias (Categoria) VALUES (?1)", [&cn])
                            .map_err(|e| e.to_string())?;
                        Some(tx.last_insert_rowid())
                    }
                }
            }
            _ => None,
        };

        // Unidad (se crea si no existe; por defecto "Pieza")
        let unidad_nombre = f
            .unidad
            .as_ref()
            .map(|u| norm(u))
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| norm("Pieza"));
        let id_uni: i64 = {
            let found: Option<i64> = tx
                .query_row(
                    "SELECT ID_UnidadMedida FROM UnidadMedida WHERE UPPER(UnidadMedida) = ?1",
                    [&unidad_nombre],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            match found {
                Some(id) => id,
                None => {
                    tx.execute(
                        "INSERT INTO UnidadMedida (UnidadMedida) VALUES (?1)",
                        [&unidad_nombre],
                    )
                    .map_err(|e| e.to_string())?;
                    tx.last_insert_rowid()
                }
            }
        };

        // ¿El producto ya existe? (por código o por nombre)
        let id_existente: Option<i64> = tx
            .query_row(
                "SELECT ID_Producto FROM Productos
                 WHERE UPPER(Producto) = ?1 OR (?2 IS NOT NULL AND CodigoBarras = ?2)",
                params![nombre, codigo],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        let id = match id_existente {
            Some(id) => {
                tx.execute(
                    "UPDATE Productos SET PrecioUnitario = ?1, PrecioCosto = ?2,
                        ID_Categoria = COALESCE(?3, ID_Categoria), ID_UnidadMedida = ?4,
                        SeVendePeso = COALESCE(?5, SeVendePeso)
                     WHERE ID_Producto = ?6",
                    params![
                        f.precio_venta,
                        f.precio_costo.unwrap_or(0.0),
                        id_cat,
                        id_uni,
                        f.se_vende_peso.map(|b| b as i64),
                        id
                    ],
                )
                .map_err(|e| e.to_string())?;
                id
            }
            None => {
                let res = tx.execute(
                    "INSERT INTO Productos
                        (Producto, CodigoBarras, PrecioUnitario, PrecioCosto, ID_Categoria, ID_UnidadMedida, SeVendePeso)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        nombre,
                        codigo,
                        f.precio_venta,
                        f.precio_costo.unwrap_or(0.0),
                        id_cat,
                        id_uni,
                        f.se_vende_peso.unwrap_or(false) as i64
                    ],
                );
                match res {
                    Ok(_) => tx.last_insert_rowid(),
                    Err(e) => {
                        omitidos += 1;
                        let msg = if e.to_string().contains("UNIQUE") {
                            "código de barras duplicado".to_string()
                        } else {
                            e.to_string()
                        };
                        errores.push(format!("Fila {renglon} ({nombre}): {msg}"));
                        continue;
                    }
                }
            }
        };

        // RESET del stock de ESTE producto (solo lo que está en el archivo)
        tx.execute("DELETE FROM CapasCosto WHERE ID_Producto = ?1", [id])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM Lotes WHERE ID_Producto = ?1", [id])
            .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM MovimientosInventario WHERE ID_Producto = ?1 AND Motivo = 'Inventario inicial'",
            [id],
        )
        .map_err(|e| e.to_string())?;

        let exist = f.existencia.unwrap_or(0.0).max(0.0);
        let costo = f.precio_costo.unwrap_or(0.0);
        let maneja: i64 = tx
            .query_row(
                "SELECT ManejaCaducidad FROM Productos WHERE ID_Producto = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if maneja != 0 {
            // Caducidad: el stock vive en lotes; Inventario queda en 0.
            tx.execute(
                "INSERT INTO Inventario (ID_Producto, Cantidad) VALUES (?1, 0)
                 ON CONFLICT(ID_Producto) DO UPDATE SET Cantidad = 0",
                [id],
            )
            .map_err(|e| e.to_string())?;
            if exist > 0.0 {
                agregar_lote(&tx, id, None, None, exist)?;
            }
        } else {
            tx.execute(
                "INSERT INTO Inventario (ID_Producto, Cantidad) VALUES (?1, ?2)
                 ON CONFLICT(ID_Producto) DO UPDATE SET Cantidad = ?2",
                params![id, exist],
            )
            .map_err(|e| e.to_string())?;
        }

        if exist > 0.0 {
            agregar_capa(&tx, id, exist, costo)?;
            tx.execute(
                "INSERT INTO MovimientosInventario (ID_Producto, Tipo, Cantidad, Motivo, CostoUnitario)
                 VALUES (?1, 'Entrada', ?2, 'Inventario inicial', ?3)",
                params![id, exist, costo],
            )
            .map_err(|e| e.to_string())?;
        }
        insertados += 1;
    }

    tx.commit().map_err(|e| e.to_string())?;
    bitacora(
        &pool,
        &sesion,
        "Inventario inicial",
        "Producto",
        None,
        &format!("{insertados} productos cargados, {omitidos} omitidos"),
    );
    Ok(ResultadoImport {
        insertados,
        omitidos,
        errores,
    })
}

/// Actualiza precios en masa desde un CSV/XLSX, SIN tocar existencias ni PEPS.
/// Empata por código de barras o por nombre. Cambia PrecioUnitario (venta) y, si el
/// archivo trae PRECIOCOSTO, también el costo de referencia. Los productos que no
/// existen se reportan (no se crean: para altas usa "Importar productos").
#[tauri::command]
pub fn actualizar_precios(
    pool: State<Db>,
    sesion: State<Sesion>,
    filas: Vec<FilaProductoImport>,
) -> Result<ResultadoImport, String> {
    exigir_admin(&sesion)?;
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut insertados = 0i64; // aquí = actualizados
    let mut omitidos = 0i64;
    let mut errores: Vec<String> = Vec::new();

    for (i, f) in filas.iter().enumerate() {
        let renglon = i + 2;
        let nombre = norm(&f.producto);
        if nombre.is_empty() {
            omitidos += 1;
            errores.push(format!("Fila {renglon}: nombre vacío"));
            continue;
        }
        if f.precio_venta < 0.0 {
            omitidos += 1;
            errores.push(format!("Fila {renglon} ({nombre}): PRECIOVENTA inválido"));
            continue;
        }
        let codigo: Option<String> = f
            .codigo_barras
            .as_ref()
            .map(|c| norm(c))
            .filter(|c| !c.is_empty());
        let id: Option<i64> = tx
            .query_row(
                "SELECT ID_Producto FROM Productos
                 WHERE UPPER(Producto) = ?1 OR (?2 IS NOT NULL AND CodigoBarras = ?2)",
                params![nombre, codigo],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        match id {
            Some(id) => {
                match f.precio_costo {
                    Some(c) if c >= 0.0 => {
                        tx.execute(
                            "UPDATE Productos SET PrecioUnitario = ?1, PrecioCosto = ?2
                             WHERE ID_Producto = ?3",
                            params![f.precio_venta, c, id],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    _ => {
                        tx.execute(
                            "UPDATE Productos SET PrecioUnitario = ?1 WHERE ID_Producto = ?2",
                            params![f.precio_venta, id],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                }
                insertados += 1;
            }
            None => {
                omitidos += 1;
                errores.push(format!("Fila {renglon} ({nombre}): no existe (no se creó)"));
            }
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    bitacora(
        &pool,
        &sesion,
        "Actualización precios",
        "Producto",
        None,
        &format!("{insertados} precios actualizados, {omitidos} no encontrados"),
    );
    Ok(ResultadoImport {
        insertados,
        omitidos,
        errores,
    })
}

#[tauri::command]
pub fn importar_categorias(
    pool: State<Db>,
    sesion: State<Sesion>,
    filas: Vec<FilaCategoriaImport>,
) -> Result<ResultadoImport, String> {
    exigir_admin(&sesion)?;
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut insertados = 0i64;
    let mut omitidos = 0i64;
    let mut errores: Vec<String> = Vec::new();

    for (i, f) in filas.iter().enumerate() {
        let nombre = norm(&f.categoria);
        if nombre.is_empty() {
            omitidos += 1;
            errores.push(format!("Fila {}: nombre vacío", i + 2));
            continue;
        }
        let existe: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM Categorias WHERE UPPER(Categoria) = ?1",
                [&nombre],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if existe > 0 {
            omitidos += 1; // ya existe, no duplicar
            continue;
        }
        let desc = f
            .descripcion
            .as_ref()
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());
        tx.execute(
            "INSERT INTO Categorias (Categoria, Descripcion) VALUES (?1, ?2)",
            params![nombre, desc],
        )
        .map_err(|e| e.to_string())?;
        insertados += 1;
    }

    tx.commit().map_err(|e| e.to_string())?;
    bitacora(
        &pool,
        &sesion,
        "Importación",
        "Categoría",
        None,
        &format!("{insertados} categorías"),
    );
    Ok(ResultadoImport {
        insertados,
        omitidos,
        errores,
    })
}

#[tauri::command]
pub fn importar_proveedores(
    pool: State<Db>,
    sesion: State<Sesion>,
    filas: Vec<FilaProveedorImport>,
) -> Result<ResultadoImport, String> {
    exigir_admin(&sesion)?;
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut insertados = 0i64;
    let mut omitidos = 0i64;
    let mut errores: Vec<String> = Vec::new();

    for (i, f) in filas.iter().enumerate() {
        let nombre = norm(&f.proveedor);
        if nombre.is_empty() {
            omitidos += 1;
            errores.push(format!("Fila {}: nombre vacío", i + 2));
            continue;
        }
        let existe: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM Proveedores WHERE UPPER(Proveedor) = ?1",
                [&nombre],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if existe > 0 {
            omitidos += 1;
            continue;
        }
        let contacto = f
            .contacto
            .as_ref()
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty());
        let tel = match f
            .telefono
            .as_ref()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
        {
            Some(t) => t,
            None => {
                omitidos += 1;
                errores.push(format!("Fila {} ({nombre}): falta TELEFONO", i + 2));
                continue;
            }
        };
        let email = f
            .email
            .as_ref()
            .map(|e| e.trim().to_string())
            .filter(|e| !e.is_empty());
        tx.execute(
            "INSERT INTO Proveedores (Proveedor, Contacto, Telefono, Email) VALUES (?1, ?2, ?3, ?4)",
            params![nombre, contacto, tel, email],
        )
        .map_err(|e| e.to_string())?;
        insertados += 1;
    }

    tx.commit().map_err(|e| e.to_string())?;
    bitacora(
        &pool,
        &sesion,
        "Importación",
        "Proveedor",
        None,
        &format!("{insertados} proveedores"),
    );
    Ok(ResultadoImport {
        insertados,
        omitidos,
        errores,
    })
}

#[tauri::command]
pub fn importar_clientes(
    pool: State<Db>,
    sesion: State<Sesion>,
    filas: Vec<FilaClienteImport>,
) -> Result<ResultadoImport, String> {
    exigir_admin(&sesion)?;
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut insertados = 0i64;
    let mut omitidos = 0i64;
    let mut errores: Vec<String> = Vec::new();

    for (i, f) in filas.iter().enumerate() {
        let renglon = i + 2;
        let nombre = norm(&f.nombre);
        if nombre.is_empty() {
            omitidos += 1;
            errores.push(format!("Fila {renglon}: nombre vacío"));
            continue;
        }
        // El teléfono es obligatorio; se recorta pero no se pasa a mayúsculas (son dígitos).
        let tel: String = match f
            .telefono
            .as_ref()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
        {
            Some(t) => t,
            None => {
                omitidos += 1;
                errores.push(format!("Fila {renglon} ({nombre}): falta TELEFONO"));
                continue;
            }
        };
        let email = f
            .email
            .as_ref()
            .map(|e| e.trim().to_string())
            .filter(|e| !e.is_empty());

        tx.execute(
            "INSERT INTO Clientes (Nombre, Telefono, Email) VALUES (?1, ?2, ?3)",
            params![nombre, tel, email],
        )
        .map_err(|e| e.to_string())?;
        insertados += 1;
    }

    tx.commit().map_err(|e| e.to_string())?;
    bitacora(
        &pool,
        &sesion,
        "Importación",
        "Cliente",
        None,
        &format!("{insertados} clientes importados"),
    );
    Ok(ResultadoImport {
        insertados,
        omitidos,
        errores,
    })
}

// ---------------------------------------------------------------------
//  Helper compartido: fila SQL -> Producto
// ---------------------------------------------------------------------
fn fila_a_producto(r: &rusqlite::Row) -> rusqlite::Result<Producto> {
    Ok(Producto {
        id_producto: r.get(0)?,
        producto: r.get(1)?,
        codigo_barras: r.get(2)?,
        precio_unitario: r.get(3)?,
        precio_costo: r.get(4)?,
        se_vende_peso: r.get::<_, i64>(5)? != 0,
        stock_minimo: r.get(6)?,
        id_categoria: r.get(7)?,
        categoria: r.get(8)?,
        existencia: r.get(9)?,
        activo: r.get::<_, i64>(10)? != 0,
        id_unidad_medida: r.get(11)?,
        tipo: r.get(12)?,
        maneja_caducidad: r.get::<_, i64>(13)? != 0,
        unidad: r.get(14)?,
    })
}

/// Consume `cantidad` de los lotes del producto en orden FEFO (la caducidad más
/// próxima primero; los lotes sin caducidad al final). Devuelve lo que no se
/// pudo cubrir (0.0 si alcanzó).
fn consumir_lotes_fefo(
    tx: &rusqlite::Transaction,
    id_producto: i64,
    cantidad: f64,
) -> Result<f64, String> {
    let lotes: Vec<(i64, f64)> = {
        let mut stmt = tx
            .prepare(
                "SELECT ID_Lote, Cantidad FROM Lotes
                 WHERE ID_Producto = ?1 AND Cantidad > 0
                 ORDER BY (Caducidad IS NULL), Caducidad ASC, ID_Lote ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([id_producto], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())?
    };
    let mut restante = cantidad;
    for (id_lote, disponible) in lotes {
        if restante <= 0.0 {
            break;
        }
        let usar = restante.min(disponible);
        tx.execute(
            "UPDATE Lotes SET Cantidad = Cantidad - ?1 WHERE ID_Lote = ?2",
            params![usar, id_lote],
        )
        .map_err(|e| e.to_string())?;
        restante -= usar;
    }
    Ok(restante)
}

/// Crea un lote para un producto con caducidad (lote/caducidad opcionales).
fn agregar_lote(
    tx: &rusqlite::Transaction,
    id_producto: i64,
    lote: Option<&str>,
    caducidad: Option<&str>,
    cantidad: f64,
) -> Result<(), String> {
    let lote = lote.map(|s| s.trim()).filter(|s| !s.is_empty());
    let caducidad = caducidad.map(|s| s.trim()).filter(|s| !s.is_empty());
    tx.execute(
        "INSERT INTO Lotes (ID_Producto, Lote, Caducidad, Cantidad) VALUES (?1, ?2, ?3, ?4)",
        params![id_producto, lote, caducidad, cantidad],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Agrega una capa de costo PEPS (de una compra o alta) para un producto.
fn agregar_capa(
    tx: &rusqlite::Transaction,
    id_producto: i64,
    cantidad: f64,
    costo_unitario: f64,
) -> Result<(), String> {
    if cantidad <= 0.0 {
        return Ok(());
    }
    tx.execute(
        "INSERT INTO CapasCosto (ID_Producto, Cantidad, CostoUnitario) VALUES (?1, ?2, ?3)",
        params![id_producto, cantidad, costo_unitario],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Consume `cantidad` de las capas PEPS (la más antigua primero) y devuelve el
/// COSTO total consumido (COGS). Si faltan capas (descuadre), valúa el resto al
/// PrecioCosto actual del producto para no subvaluar el costo.
fn consumir_capas_fifo(
    tx: &rusqlite::Transaction,
    id_producto: i64,
    cantidad: f64,
) -> Result<f64, String> {
    if cantidad <= 0.0 {
        return Ok(0.0);
    }
    let capas: Vec<(i64, f64, f64)> = {
        let mut stmt = tx
            .prepare(
                "SELECT ID_Capa, Cantidad, CostoUnitario FROM CapasCosto
                 WHERE ID_Producto = ?1 AND Cantidad > 0
                 ORDER BY ID_Capa ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([id_producto], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())?
    };
    let mut restante = cantidad;
    let mut costo_total = 0.0_f64;
    for (id_capa, disponible, costo) in capas {
        if restante <= 0.0 {
            break;
        }
        let usar = restante.min(disponible);
        costo_total += usar * costo;
        tx.execute(
            "UPDATE CapasCosto SET Cantidad = Cantidad - ?1 WHERE ID_Capa = ?2",
            params![usar, id_capa],
        )
        .map_err(|e| e.to_string())?;
        restante -= usar;
    }
    if restante > 0.0 {
        let ultimo: f64 = tx
            .query_row(
                "SELECT PrecioCosto FROM Productos WHERE ID_Producto = ?1",
                [id_producto],
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        costo_total += restante * ultimo;
    }
    Ok(costo_total)
}

/// Refleja en las capas PEPS un ajuste de inventario: un sobrante crea una capa
/// al costo actual; un faltante consume capas (la pérdida sale del valor).
/// Devuelve el COSTO total involucrado (agregado o consumido), para registrarlo
/// como costo unitario del movimiento en el kardex.
fn reflejar_capas_ajuste(
    tx: &rusqlite::Transaction,
    id_producto: i64,
    delta: f64,
    costo_provisto: Option<f64>,
) -> Result<f64, String> {
    if delta > 0.0 {
        // Stock "encontrado": costo que indique el usuario; si no, el del ÚLTIMO
        // lote/capa registrado; si no hay capas, el PrecioCosto; si tampoco, 0.
        let costo = if let Some(c) = costo_provisto.filter(|c| *c > 0.0) {
            c
        } else {
            tx.query_row(
                "SELECT CostoUnitario FROM CapasCosto
                 WHERE ID_Producto = ?1 AND CostoUnitario > 0
                 ORDER BY ID_Capa DESC LIMIT 1",
                [id_producto],
                |r| r.get::<_, f64>(0),
            )
            .or_else(|_| {
                tx.query_row(
                    "SELECT PrecioCosto FROM Productos WHERE ID_Producto = ?1",
                    [id_producto],
                    |r| r.get::<_, f64>(0),
                )
            })
            .unwrap_or(0.0)
        };
        agregar_capa(tx, id_producto, delta, costo)?;
        Ok(delta * costo)
    } else if delta < 0.0 {
        // Merma/faltante: descuenta del lote más viejo (FEFO/FIFO), como una venta.
        consumir_capas_fifo(tx, id_producto, -delta)
    } else {
        Ok(0.0)
    }
}

/// Edita los datos de un producto (no toca la existencia; eso va por ajuste).
#[tauri::command]
pub fn actualizar_producto(
    pool: State<Db>,
    sesion: State<Sesion>,
    id: i64,
    datos: EditarProducto,
) -> Result<(), String> {
    exigir_admin(&sesion)?;
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let es_servicio = datos.tipo.eq_ignore_ascii_case("Servicio");
    let maneja = !es_servicio && datos.maneja_caducidad;

    // Modo de caducidad anterior, para migrar el stock si cambia.
    let antes: bool = tx
        .query_row(
            "SELECT ManejaCaducidad FROM Productos WHERE ID_Producto = ?1",
            [id],
            |r| r.get::<_, i64>(0),
        )
        .map_err(|_| "Producto no encontrado".to_string())?
        != 0;

    tx.execute(
        "UPDATE Productos
         SET Producto = ?1, CodigoBarras = ?2, PrecioUnitario = ?3, PrecioCosto = ?4,
             Tipo = ?5, ManejaCaducidad = ?6, SeVendePeso = ?7, StockMinimo = ?8,
             ID_UnidadMedida = ?9, ID_Categoria = ?10, Activo = ?11
         WHERE ID_Producto = ?12",
        params![
            datos.producto,
            datos.codigo_barras,
            datos.precio_unitario,
            datos.precio_costo,
            if es_servicio { "Servicio" } else { "Producto" },
            maneja as i64,
            datos.se_vende_peso as i64,
            datos.stock_minimo,
            datos.id_unidad_medida,
            datos.id_categoria,
            datos.activo as i64,
            id,
        ],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Ya existe un producto con ese código de barras".to_string()
        } else {
            e.to_string()
        }
    })?;

    // Si cambió el modo de caducidad, migra el stock para no "perder" existencia.
    if maneja != antes {
        if maneja {
            // Inventario -> un lote (sin caducidad); el admin ajustará luego.
            let actual: f64 = tx
                .query_row(
                    "SELECT COALESCE(Cantidad, 0) FROM Inventario WHERE ID_Producto = ?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?
                .unwrap_or(0.0);
            if actual > 0.0 {
                agregar_lote(&tx, id, None, None, actual)?;
            }
            tx.execute(
                "UPDATE Inventario SET Cantidad = 0 WHERE ID_Producto = ?1",
                [id],
            )
            .map_err(|e| e.to_string())?;
        } else {
            // Lotes -> Inventario; se vacían los lotes.
            let suma: f64 = tx
                .query_row(
                    "SELECT COALESCE(SUM(Cantidad), 0) FROM Lotes WHERE ID_Producto = ?1",
                    [id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO Inventario (ID_Producto, Cantidad) VALUES (?1, ?2)
                 ON CONFLICT(ID_Producto) DO UPDATE SET Cantidad = ?2",
                params![id, suma],
            )
            .map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM Lotes WHERE ID_Producto = ?1", [id])
                .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    bitacora(&pool, &sesion, "Edición", "Producto", Some(id), &datos.producto);
    Ok(())
}

/// Ajusta la existencia de un producto y deja el movimiento en el kardex.
///   - 'Entrada': suma `cantidad`
///   - 'Merma':   resta `cantidad`
///   - 'Ajuste':  fija la existencia a `cantidad` (conteo físico)
/// Devuelve la nueva existencia.
#[tauri::command]
pub fn ajustar_inventario(
    pool: State<Db>,
    sesion: State<Sesion>,
    id_producto: i64,
    tipo: String,
    cantidad: f64,
    motivo: Option<String>,
    id_usuario: Option<i64>,
    costo: Option<f64>,
) -> Result<f64, String> {
    exigir_admin(&sesion)?;
    if cantidad < 0.0 {
        return Err("La cantidad no puede ser negativa".into());
    }
    // El motivo es OBLIGATORIO: todo aumento/baja manual debe quedar justificado
    // para tener trazabilidad exacta en el kardex.
    let motivo = motivo
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .ok_or_else(|| "Indica el motivo del ajuste".to_string())?;

    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // Si el producto maneja caducidad, el stock vive en lotes (no en Inventario).
    let maneja: i64 = tx
        .query_row(
            "SELECT ManejaCaducidad FROM Productos WHERE ID_Producto = ?1",
            [id_producto],
            |r| r.get(0),
        )
        .map_err(|_| "Producto no encontrado".to_string())?;
    if maneja != 0 {
        let suma: f64 = tx
            .query_row(
                "SELECT COALESCE(SUM(Cantidad), 0) FROM Lotes WHERE ID_Producto = ?1",
                [id_producto],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let delta = match tipo.as_str() {
            "Entrada" => cantidad,
            "Merma" => -cantidad,
            "Ajuste" => cantidad - suma,
            _ => return Err("Tipo inválido (Entrada/Merma/Ajuste)".into()),
        };
        if suma + delta < 0.0 {
            return Err("La existencia no puede quedar negativa".into());
        }
        if delta > 0.0 {
            // Entrada rápida sin caducidad; para caducidad usa Compras.
            agregar_lote(&tx, id_producto, None, None, delta)?;
        } else if delta < 0.0 {
            let faltante = consumir_lotes_fefo(&tx, id_producto, -delta)?;
            if faltante > 0.0 {
                return Err("No hay suficiente existencia en los lotes".into());
            }
        }
        let costo_aj = reflejar_capas_ajuste(&tx, id_producto, delta, costo)?;
        tx.execute(
            "INSERT INTO MovimientosInventario (ID_Producto, Tipo, Cantidad, Motivo, ID_Usuario, CostoUnitario)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id_producto,
                tipo,
                delta,
                motivo,
                id_usuario,
                if delta != 0.0 { costo_aj / delta.abs() } else { 0.0 }
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        bitacora(
            &pool,
            &sesion,
            "Ajuste",
            "Inventario",
            Some(id_producto),
            &format!("{tipo}: {cantidad} (lotes)"),
        );
        return Ok(suma + delta);
    }

    tx.execute(
        "INSERT INTO Inventario (ID_Producto, Cantidad) VALUES (?1, 0)
         ON CONFLICT(ID_Producto) DO NOTHING",
        params![id_producto],
    )
    .map_err(|e| e.to_string())?;

    let actual: f64 = tx
        .query_row(
            "SELECT Cantidad FROM Inventario WHERE ID_Producto = ?1",
            [id_producto],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let delta = match tipo.as_str() {
        "Entrada" => cantidad,
        "Merma" => -cantidad,
        "Ajuste" => cantidad - actual,
        _ => return Err("Tipo inválido (Entrada/Merma/Ajuste)".into()),
    };

    let nueva = actual + delta;
    if nueva < 0.0 {
        return Err("La existencia no puede quedar negativa".into());
    }

    tx.execute(
        "UPDATE Inventario SET Cantidad = ?1, FechaModificacion = CURRENT_TIMESTAMP
         WHERE ID_Producto = ?2",
        params![nueva, id_producto],
    )
    .map_err(|e| e.to_string())?;
    let costo_aj = reflejar_capas_ajuste(&tx, id_producto, delta, costo)?;

    tx.execute(
        "INSERT INTO MovimientosInventario (ID_Producto, Tipo, Cantidad, Motivo, ID_Usuario, CostoUnitario)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id_producto,
            tipo,
            delta,
            motivo,
            id_usuario,
            if delta != 0.0 { costo_aj / delta.abs() } else { 0.0 }
        ],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    bitacora(
        &pool,
        &sesion,
        "Ajuste",
        "Inventario",
        Some(id_producto),
        &format!("{tipo}: {cantidad}"),
    );
    Ok(nueva)
}

/// Reporte de entradas/salidas de mercancía (kardex) agrupado por producto,
/// entre dos fechas ('YYYY-MM-DD', inclusivo, hora local). Cuenta lo comprado,
/// vendido, mermado y los totales de entradas/salidas en el periodo.
#[tauri::command]
pub fn reporte_movimientos(
    pool: State<Db>,
    desde: String,
    hasta: String,
) -> Result<Vec<MovimientoResumen>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT p.Producto,
                    COALESCE(SUM(CASE WHEN m.Motivo = 'Compra' THEN m.Cantidad ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN m.Tipo = 'Venta'  THEN -m.Cantidad ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN m.Tipo = 'Merma'  THEN -m.Cantidad ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN m.Cantidad > 0 THEN m.Cantidad  ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN m.Cantidad < 0 THEN -m.Cantidad ELSE 0 END), 0)
             FROM MovimientosInventario m
             JOIN Productos p ON p.ID_Producto = m.ID_Producto
             WHERE date(m.Fecha, 'localtime') BETWEEN ?1 AND ?2
             GROUP BY m.ID_Producto, p.Producto
             ORDER BY p.Producto",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![desde, hasta], |r| {
            Ok(MovimientoResumen {
                producto: r.get(0)?,
                comprado: r.get(1)?,
                vendido: r.get(2)?,
                merma: r.get(3)?,
                entradas: r.get(4)?,
                salidas: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// Historial (kardex) completo de un producto: cada movimiento con su fecha,
/// tipo, cantidad (con signo), motivo y usuario. Más reciente primero.
#[tauri::command]
pub fn historial_producto(
    pool: State<Db>,
    id_producto: i64,
) -> Result<Vec<MovimientoDetalle>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT m.Fecha, m.Tipo, m.Cantidad, m.Motivo, u.Nombre
             FROM MovimientosInventario m
             LEFT JOIN Usuarios u ON u.ID_Usuario = m.ID_Usuario
             WHERE m.ID_Producto = ?1
             ORDER BY m.ID_Movimiento DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([id_producto], |r| {
            Ok(MovimientoDetalle {
                fecha: r.get(0)?,
                tipo: r.get(1)?,
                cantidad: r.get(2)?,
                motivo: r.get(3)?,
                usuario: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// Reporte de utilidad (PEPS) entre dos fechas: por producto, lo vendido, las
/// ventas, el costo de ventas (COGS PEPS), la utilidad y el margen %.
#[tauri::command]
pub fn reporte_utilidad(
    pool: State<Db>,
    desde: String,
    hasta: String,
) -> Result<Vec<UtilidadProducto>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT p.Producto,
                    SUM(dv.Cantidad),
                    SUM(dv.Cantidad * dv.PrecioVentaHistorico),
                    SUM(COALESCE(dv.CostoHistorico, 0))
             FROM Detalle_Ventas dv
             JOIN Ventas v ON v.ID_Venta = dv.ID_Venta
             JOIN Productos p ON p.ID_Producto = dv.ID_Producto
             WHERE v.Estatus = 'Completada'
               AND date(v.FechaVenta, 'localtime') BETWEEN ?1 AND ?2
             GROUP BY dv.ID_Producto, p.Producto
             ORDER BY p.Producto",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![desde, hasta], |r| {
            let producto: String = r.get(0)?;
            let vendido: f64 = r.get(1)?;
            let ventas: f64 = r.get(2)?;
            let costo: f64 = r.get(3)?;
            let utilidad = ventas - costo;
            let margen = if ventas > 0.0 { utilidad / ventas * 100.0 } else { 0.0 };
            Ok(UtilidadProducto {
                producto,
                vendido,
                ventas,
                costo,
                utilidad,
                margen,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// Inventario valuado a PEPS: por producto, existencia, costo unitario (de las
/// capas restantes) y valor total. Solo productos activos con existencia/valor.
#[tauri::command]
pub fn reporte_inventario_valorizado(
    pool: State<Db>,
) -> Result<Vec<InventarioValorizado>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT p.Producto,
                    CASE WHEN p.ManejaCaducidad = 1
                         THEN COALESCE((SELECT SUM(Cantidad) FROM Lotes WHERE ID_Producto = p.ID_Producto), 0)
                         ELSE COALESCE((SELECT Cantidad FROM Inventario WHERE ID_Producto = p.ID_Producto), 0) END,
                    COALESCE((SELECT SUM(Cantidad * CostoUnitario) FROM CapasCosto WHERE ID_Producto = p.ID_Producto), 0)
             FROM Productos p
             WHERE p.Tipo <> 'Servicio' AND p.Activo = 1
             ORDER BY p.Producto",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            let producto: String = r.get(0)?;
            let existencia: f64 = r.get(1)?;
            let valor: f64 = r.get(2)?;
            let costo_unitario = if existencia > 0.0 { valor / existencia } else { 0.0 };
            Ok(InventarioValorizado {
                producto,
                existencia,
                costo_unitario,
                valor,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
        .map(|v: Vec<InventarioValorizado>| {
            v.into_iter()
                .filter(|x| x.existencia > 0.0 || x.valor.abs() > 0.0001)
                .collect()
        })
}

/// Kardex valorizado (tarjeta de almacén) de un producto: cada movimiento con
/// su costo unitario y el saldo acumulado (cantidad y valor PEPS). Más antiguo
/// primero, para que el saldo se lea de arriba hacia abajo.
#[tauri::command]
pub fn kardex_valorizado(
    pool: State<Db>,
    id_producto: i64,
) -> Result<Vec<KardexLinea>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT m.Fecha, COALESCE(NULLIF(m.Motivo, ''), m.Tipo), m.Cantidad,
                    COALESCE(m.CostoUnitario, 0)
             FROM MovimientosInventario m
             WHERE m.ID_Producto = ?1
             ORDER BY m.ID_Movimiento ASC",
        )
        .map_err(|e| e.to_string())?;
    let filas: Vec<(String, String, f64, f64)> = stmt
        .query_map([id_producto], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    let mut saldo_c = 0.0_f64;
    let mut saldo_v = 0.0_f64;
    let mut out = Vec::with_capacity(filas.len());
    for (fecha, concepto, cantidad, costo) in filas {
        saldo_c += cantidad;
        saldo_v += cantidad * costo;
        out.push(KardexLinea {
            fecha,
            concepto,
            cantidad,
            costo_unitario: costo,
            saldo_cantidad: saldo_c,
            saldo_valor: saldo_v,
        });
    }
    Ok(out)
}

// =====================================================================
//  LOTES / CADUCIDAD
// =====================================================================

/// Lotes con existencia de un producto, ordenados por caducidad (FEFO).
#[tauri::command]
pub fn listar_lotes(pool: State<Db>, id_producto: i64) -> Result<Vec<Lote>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT ID_Lote, ID_Producto, Lote, Caducidad, Cantidad FROM Lotes
             WHERE ID_Producto = ?1 AND Cantidad > 0
             ORDER BY (Caducidad IS NULL), Caducidad ASC, ID_Lote ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([id_producto], |r| {
            Ok(Lote {
                id_lote: r.get(0)?,
                id_producto: r.get(1)?,
                lote: r.get(2)?,
                caducidad: r.get(3)?,
                cantidad: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// Lotes con caducidad que vencen en `dias` días o menos (incluye vencidos).
/// Pensado para la pantalla de alertas de caducidad.
#[tauri::command]
pub fn lotes_por_vencer(pool: State<Db>, dias: i64) -> Result<Vec<LoteVencimiento>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT l.ID_Lote, l.ID_Producto, p.Producto, l.Lote, l.Caducidad, l.Cantidad,
                    CAST(julianday(l.Caducidad) - julianday(date('now','localtime')) AS INTEGER)
             FROM Lotes l
             JOIN Productos p ON p.ID_Producto = l.ID_Producto
             WHERE l.Cantidad > 0 AND l.Caducidad IS NOT NULL
               AND julianday(l.Caducidad) - julianday(date('now','localtime')) <= ?1
             ORDER BY l.Caducidad ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([dias], |r| {
            Ok(LoteVencimiento {
                id_lote: r.get(0)?,
                id_producto: r.get(1)?,
                producto: r.get(2)?,
                lote: r.get(3)?,
                caducidad: r.get(4)?,
                cantidad: r.get(5)?,
                dias_restantes: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// Da de baja un lote (merma): pone su cantidad en 0 y registra el movimiento.
#[tauri::command]
pub fn dar_baja_lote(
    pool: State<Db>,
    sesion: State<Sesion>,
    id_lote: i64,
    motivo: Option<String>,
    id_usuario: Option<i64>,
) -> Result<(), String> {
    exigir_admin(&sesion)?;
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let (id_producto, cantidad): (i64, f64) = tx
        .query_row(
            "SELECT ID_Producto, Cantidad FROM Lotes WHERE ID_Lote = ?1",
            [id_lote],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "Lote no encontrado".to_string())?;
    if cantidad <= 0.0 {
        return Err("El lote ya no tiene existencia".into());
    }
    tx.execute("UPDATE Lotes SET Cantidad = 0 WHERE ID_Lote = ?1", [id_lote])
        .map_err(|e| e.to_string())?;
    // Consume las capas PEPS de lo dado de baja (para que el valor cuadre).
    let cogs = consumir_capas_fifo(&tx, id_producto, cantidad)?;
    tx.execute(
        "INSERT INTO MovimientosInventario (ID_Producto, Tipo, Cantidad, Motivo, ID_Usuario, CostoUnitario)
         VALUES (?1, 'Merma', ?2, ?3, ?4, ?5)",
        params![
            id_producto,
            -cantidad,
            motivo.unwrap_or_else(|| "Merma de lote vencido".to_string()),
            id_usuario,
            if cantidad > 0.0 { cogs / cantidad } else { 0.0 }
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    bitacora(
        &pool,
        &sesion,
        "Merma",
        "Inventario",
        Some(id_producto),
        &format!("Baja de lote #{id_lote}: {cantidad}"),
    );
    Ok(())
}

// =====================================================================
//  USUARIOS / LOGIN  (contraseñas con Argon2)
// =====================================================================

fn hash_password(pass: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pass.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

fn verify_password(pass: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(pass.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

#[tauri::command]
pub fn crear_usuario(
    pool: State<Db>,
    sesion: State<Sesion>,
    datos: NuevoUsuario,
) -> Result<i64, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;

    // El PRIMER usuario (setup inicial) se permite sin sesión. A partir de
    // ahí, solo un administrador puede crear usuarios.
    let cuantos: i64 = conn
        .query_row("SELECT COUNT(*) FROM Usuarios", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if cuantos > 0 {
        exigir_admin(&sesion)?;
    }

    let hash = hash_password(&datos.contrasena)?;
    conn.execute(
        "INSERT INTO Usuarios (Nombre, Usuario, Contrasena, Rol) VALUES (?1, ?2, ?3, ?4)",
        params![datos.nombre, datos.usuario, hash, datos.rol],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Ese nombre de usuario ya existe".to_string()
        } else {
            e.to_string()
        }
    })?;
    let id = conn.last_insert_rowid();
    bitacora(
        &pool,
        &sesion,
        "Alta",
        "Usuario",
        Some(id),
        &format!("{} ({})", datos.usuario, datos.rol),
    );
    Ok(id)
}

/// Valida usuario+contraseña. Devuelve los datos del usuario (sin el hash)
/// o un error genérico (no revela si falló el usuario o la contraseña).
#[tauri::command]
pub fn login(
    pool: State<Db>,
    sesion: State<Sesion>,
    usuario: String,
    contrasena: String,
) -> Result<Usuario, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;

    let datos: Option<(i64, String, String, String, i64)> = conn
        .query_row(
            "SELECT ID_Usuario, Nombre, Rol, Contrasena, Activo
             FROM Usuarios WHERE Usuario = ?1",
            [&usuario],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let (id, nombre, rol, hash, activo) = match datos {
        Some(d) => d,
        None => {
            bitacora(
                &pool,
                &sesion,
                "Acceso fallido",
                "Usuario",
                None,
                &format!("Usuario inexistente: '{usuario}'"),
            );
            return Err("Usuario o contraseña incorrectos".into());
        }
    };

    if activo == 0 {
        bitacora(
            &pool,
            &sesion,
            "Acceso fallido",
            "Usuario",
            Some(id),
            &format!("Usuario inactivo: '{usuario}'"),
        );
        return Err("Usuario inactivo".into());
    }
    if !verify_password(&contrasena, &hash) {
        bitacora(
            &pool,
            &sesion,
            "Acceso fallido",
            "Usuario",
            Some(id),
            &format!("Contraseña incorrecta: '{usuario}'"),
        );
        return Err("Usuario o contraseña incorrectos".into());
    }

    // Guarda la sesión en el backend (fuente de verdad del rol).
    *sesion.0.lock().map_err(|_| "Error de sesión".to_string())? = Some(UsuarioSesion {
        id_usuario: id,
        usuario: usuario.clone(),
        nombre: nombre.clone(),
        rol: rol.clone(),
    });
    bitacora(&pool, &sesion, "Inicio de sesión", "Usuario", Some(id), &nombre);

    Ok(Usuario {
        id_usuario: id,
        nombre,
        usuario,
        rol,
        activo: true,
    })
}

/// Devuelve la sesión activa del backend, si la hay. El frontend la consulta al
/// arrancar para restaurar la sesión tras recargar (Ctrl+R): el proceso de Rust
/// no se reinicia con la recarga, así que la sesión sigue viva aquí.
#[tauri::command]
pub fn sesion_actual(sesion: State<Sesion>) -> Option<Usuario> {
    let guard = sesion.0.lock().ok()?;
    guard.as_ref().map(|u| Usuario {
        id_usuario: u.id_usuario,
        nombre: u.nombre.clone(),
        usuario: u.usuario.clone(),
        rol: u.rol.clone(),
        activo: true,
    })
}

/// Cierra la sesión del backend (el frontend lo llama al salir).
#[tauri::command]
pub fn cerrar_sesion(pool: State<Db>, sesion: State<Sesion>) {
    bitacora(&pool, &sesion, "Cierre de sesión", "Usuario", None, "");
    if let Ok(mut s) = sesion.0.lock() {
        *s = None;
    }
}

#[tauri::command]
pub fn listar_usuarios(pool: State<Db>) -> Result<Vec<Usuario>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT ID_Usuario, Nombre, Usuario, Rol, Activo
             FROM Usuarios ORDER BY Nombre",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Usuario {
                id_usuario: r.get(0)?,
                nombre: r.get(1)?,
                usuario: r.get(2)?,
                rol: r.get(3)?,
                activo: r.get::<_, i64>(4)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// Edita datos de un usuario (no la contraseña). Impide quedarse sin
/// ningún administrador activo.
#[tauri::command]
pub fn actualizar_usuario(
    pool: State<Db>,
    sesion: State<Sesion>,
    id: i64,
    datos: EditarUsuario,
) -> Result<(), String> {
    exigir_admin(&sesion)?;
    let conn = pool.get().map_err(|e| e.to_string())?;

    let (rol_actual, activo_actual): (String, i64) = conn
        .query_row(
            "SELECT Rol, Activo FROM Usuarios WHERE ID_Usuario = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "Usuario no encontrado".to_string())?;

    // Si este usuario es el último admin activo, no permitir degradarlo/desactivarlo.
    let era_admin_activo = rol_actual == "Administrador" && activo_actual != 0;
    let sera_admin_activo = datos.rol == "Administrador" && datos.activo;
    if era_admin_activo && !sera_admin_activo {
        let otros: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM Usuarios
                 WHERE Rol = 'Administrador' AND Activo = 1 AND ID_Usuario <> ?1",
                [id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if otros == 0 {
            return Err("Debe quedar al menos un administrador activo.".into());
        }
    }

    conn.execute(
        "UPDATE Usuarios SET Nombre = ?1, Usuario = ?2, Rol = ?3, Activo = ?4,
             FechaModificacion = CURRENT_TIMESTAMP
         WHERE ID_Usuario = ?5",
        params![datos.nombre, datos.usuario, datos.rol, datos.activo as i64, id],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Ese nombre de usuario ya existe".to_string()
        } else {
            e.to_string()
        }
    })?;
    bitacora(&pool, &sesion, "Edición", "Usuario", Some(id), &datos.usuario);
    Ok(())
}

/// Restablece la contraseña de un usuario (admin).
#[tauri::command]
pub fn cambiar_contrasena(
    pool: State<Db>,
    sesion: State<Sesion>,
    id: i64,
    nueva: String,
) -> Result<(), String> {
    exigir_admin(&sesion)?;
    if nueva.len() < 4 {
        return Err("La contraseña debe tener al menos 4 caracteres.".into());
    }
    let hash = hash_password(&nueva)?;
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE Usuarios SET Contrasena = ?1, FechaModificacion = CURRENT_TIMESTAMP
         WHERE ID_Usuario = ?2",
        params![hash, id],
    )
    .map_err(|e| e.to_string())?;
    bitacora(&pool, &sesion, "Contraseña", "Usuario", Some(id), "Cambio de contraseña");
    Ok(())
}

/// Consulta la bitácora de auditoría entre dos fechas (solo admin).
#[tauri::command]
pub fn listar_bitacora(
    pool: State<Db>,
    sesion: State<Sesion>,
    desde: String,
    hasta: String,
) -> Result<Vec<LogBitacora>, String> {
    exigir_admin(&sesion)?;
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT ID_Bitacora, Usuario, Accion, Entidad, Detalle, Fecha
             FROM Bitacora WHERE date(Fecha, 'localtime') BETWEEN ?1 AND ?2
             ORDER BY ID_Bitacora DESC LIMIT 500",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![desde, hasta], |r| {
            Ok(LogBitacora {
                id_bitacora: r.get(0)?,
                usuario: r.get(1)?,
                accion: r.get(2)?,
                entidad: r.get(3)?,
                detalle: r.get(4)?,
                fecha: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

// =====================================================================
//  CAJA  (cortes y movimientos de efectivo)
// =====================================================================

/// Abre un corte para el usuario. Si ya tiene uno abierto, devuelve ese
/// (no permite dos cajas abiertas a la vez por el mismo usuario).
#[tauri::command]
pub fn abrir_corte(pool: State<Db>, id_usuario: i64, monto_inicial: f64) -> Result<i64, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let abierto: Option<i64> = conn
        .query_row(
            "SELECT ID_Corte FROM CortesCaja WHERE ID_Usuario = ?1 AND Estatus = 'Abierto'",
            [id_usuario],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if let Some(id) = abierto {
        return Ok(id);
    }

    conn.execute(
        "INSERT INTO CortesCaja (ID_Usuario, MontoInicial) VALUES (?1, ?2)",
        params![id_usuario, monto_inicial],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

/// Corte abierto actual del usuario (None si no tiene). Para saber al iniciar
/// si hay caja abierta y poder asignar ID_Corte a las ventas.
#[tauri::command]
pub fn corte_abierto(pool: State<Db>, id_usuario: i64) -> Result<Option<Corte>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT ID_Corte, ID_Usuario, FechaApertura, MontoInicial, Estatus
         FROM CortesCaja
         WHERE ID_Usuario = ?1 AND Estatus = 'Abierto'
         ORDER BY ID_Corte DESC LIMIT 1",
        [id_usuario],
        |r| {
            Ok(Corte {
                id_corte: r.get(0)?,
                id_usuario: r.get(1)?,
                fecha_apertura: r.get(2)?,
                monto_inicial: r.get(3)?,
                estatus: r.get(4)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn registrar_movimiento_caja(
    pool: State<Db>,
    id_corte: i64,
    tipo: String,
    monto: f64,
    concepto: Option<String>,
) -> Result<i64, String> {
    if tipo != "Ingreso" && tipo != "Retiro" {
        return Err("El tipo debe ser 'Ingreso' o 'Retiro'".into());
    }
    if monto <= 0.0 {
        return Err("El monto debe ser mayor a cero".into());
    }
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO MovimientosCaja (ID_Corte, Tipo, Monto, Concepto) VALUES (?1, ?2, ?3, ?4)",
        params![id_corte, tipo, monto, concepto],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

/// Cierra el corte: calcula el efectivo esperado (fondo + ventas en efectivo
/// + ingresos - retiros), la diferencia contra lo contado, y lo deja cerrado.
#[tauri::command]
pub fn cerrar_corte(
    pool: State<Db>,
    sesion: State<Sesion>,
    id_corte: i64,
    monto_contado: f64,
) -> Result<ResumenCorte, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let monto_inicial: f64 = tx
        .query_row(
            "SELECT MontoInicial FROM CortesCaja WHERE ID_Corte = ?1 AND Estatus = 'Abierto'",
            [id_corte],
            |r| r.get(0),
        )
        .map_err(|_| "El corte no existe o ya está cerrado".to_string())?;

    let ventas_efectivo: f64 = tx
        .query_row(
            "SELECT COALESCE(SUM(v.Total), 0)
             FROM Ventas v
             JOIN MetodosPago m ON m.ID_MetodoPago = v.ID_MetodoPago
             WHERE v.ID_Corte = ?1 AND v.Estatus = 'Completada' AND m.MetodoPago = 'Efectivo'",
            [id_corte],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let ingresos: f64 = tx
        .query_row(
            "SELECT COALESCE(SUM(Monto), 0) FROM MovimientosCaja
             WHERE ID_Corte = ?1 AND Tipo = 'Ingreso'",
            [id_corte],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let retiros: f64 = tx
        .query_row(
            "SELECT COALESCE(SUM(Monto), 0) FROM MovimientosCaja
             WHERE ID_Corte = ?1 AND Tipo = 'Retiro'",
            [id_corte],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let monto_esperado = monto_inicial + ventas_efectivo + ingresos - retiros;
    let diferencia = monto_contado - monto_esperado;

    tx.execute(
        "UPDATE CortesCaja
         SET FechaCierre = CURRENT_TIMESTAMP, MontoEsperado = ?1, MontoContado = ?2,
             Diferencia = ?3, Estatus = 'Cerrado'
         WHERE ID_Corte = ?4",
        params![monto_esperado, monto_contado, diferencia, id_corte],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    bitacora(
        &pool,
        &sesion,
        "Cierre",
        "Corte",
        Some(id_corte),
        &format!("Esperado {monto_esperado:.2}, contado {monto_contado:.2}, dif {diferencia:.2}"),
    );

    Ok(ResumenCorte {
        id_corte,
        monto_inicial,
        ventas_efectivo,
        ingresos,
        retiros,
        monto_esperado,
        monto_contado,
        diferencia,
    })
}

/// Totales del corte en vivo (sin cerrarlo) para mostrarlos en pantalla.
#[tauri::command]
pub fn resumen_corte(pool: State<Db>, id_corte: i64) -> Result<ResumenParcialCorte, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;

    let monto_inicial: f64 = conn
        .query_row(
            "SELECT MontoInicial FROM CortesCaja WHERE ID_Corte = ?1",
            [id_corte],
            |r| r.get(0),
        )
        .map_err(|_| "Corte no encontrado".to_string())?;

    let ventas_efectivo: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(v.Total), 0)
             FROM Ventas v
             JOIN MetodosPago m ON m.ID_MetodoPago = v.ID_MetodoPago
             WHERE v.ID_Corte = ?1 AND v.Estatus = 'Completada' AND m.MetodoPago = 'Efectivo'",
            [id_corte],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let ingresos: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(Monto), 0) FROM MovimientosCaja WHERE ID_Corte = ?1 AND Tipo = 'Ingreso'",
            [id_corte],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let retiros: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(Monto), 0) FROM MovimientosCaja WHERE ID_Corte = ?1 AND Tipo = 'Retiro'",
            [id_corte],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(ResumenParcialCorte {
        id_corte,
        monto_inicial,
        ventas_efectivo,
        ingresos,
        retiros,
        monto_esperado: monto_inicial + ventas_efectivo + ingresos - retiros,
    })
}

#[tauri::command]
pub fn listar_movimientos_caja(
    pool: State<Db>,
    id_corte: i64,
) -> Result<Vec<MovimientoCaja>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT ID_Movimiento, Tipo, Monto, Concepto, Fecha
             FROM MovimientosCaja WHERE ID_Corte = ?1 ORDER BY ID_Movimiento DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([id_corte], |r| {
            Ok(MovimientoCaja {
                id_movimiento: r.get(0)?,
                tipo: r.get(1)?,
                monto: r.get(2)?,
                concepto: r.get(3)?,
                fecha: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// Cortes en un rango de fechas (para reportes).
#[tauri::command]
pub fn listar_cortes(
    pool: State<Db>,
    desde: String,
    hasta: String,
) -> Result<Vec<CorteHistorico>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT c.ID_Corte, u.Nombre, c.FechaApertura, c.FechaCierre, c.MontoInicial,
                    c.MontoEsperado, c.MontoContado, c.Diferencia, c.Estatus
             FROM CortesCaja c
             JOIN Usuarios u ON u.ID_Usuario = c.ID_Usuario
             WHERE date(c.FechaApertura, 'localtime') BETWEEN ?1 AND ?2
             ORDER BY c.ID_Corte DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![desde, hasta], |r| {
            Ok(CorteHistorico {
                id_corte: r.get(0)?,
                usuario: r.get(1)?,
                fecha_apertura: r.get(2)?,
                fecha_cierre: r.get(3)?,
                monto_inicial: r.get(4)?,
                monto_esperado: r.get(5)?,
                monto_contado: r.get(6)?,
                diferencia: r.get(7)?,
                estatus: r.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

// =====================================================================
//  PROVEEDORES y COMPRAS
// =====================================================================

#[tauri::command]
pub fn listar_proveedores(
    pool: State<Db>,
    incluir_inactivos: Option<bool>,
) -> Result<Vec<Proveedor>, String> {
    let todos = incluir_inactivos.unwrap_or(false) as i64;
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT ID_Proveedor, Proveedor, Contacto, Telefono, Email, Activo
             FROM Proveedores WHERE (?1 = 1 OR Activo = 1) ORDER BY Proveedor",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([todos], |r| {
            Ok(Proveedor {
                id_proveedor: r.get(0)?,
                proveedor: r.get(1)?,
                contacto: r.get(2)?,
                telefono: r.get(3)?,
                email: r.get(4)?,
                activo: r.get::<_, i64>(5)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn crear_proveedor(
    pool: State<Db>,
    sesion: State<Sesion>,
    datos: NuevoProveedor,
) -> Result<i64, String> {
    exigir_admin(&sesion)?;
    let proveedor = datos.proveedor.trim();
    if proveedor.is_empty() {
        return Err("El nombre del proveedor es obligatorio".into());
    }
    let telefono = datos
        .telefono
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| "El teléfono es obligatorio".to_string())?;
    let contacto = datos.contacto.as_deref().map(str::trim).filter(|c| !c.is_empty());
    let email = datos.email.as_deref().map(str::trim).filter(|e| !e.is_empty());
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO Proveedores (Proveedor, Contacto, Telefono, Email) VALUES (?1, ?2, ?3, ?4)",
        params![proveedor, contacto, telefono, email],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    bitacora(&pool, &sesion, "Alta", "Proveedor", Some(id), proveedor);
    Ok(id)
}

#[tauri::command]
pub fn actualizar_proveedor(
    pool: State<Db>,
    sesion: State<Sesion>,
    id: i64,
    datos: EditarProveedor,
) -> Result<(), String> {
    exigir_admin(&sesion)?;
    let proveedor = datos.proveedor.trim();
    if proveedor.is_empty() {
        return Err("El nombre del proveedor es obligatorio".into());
    }
    let telefono = datos
        .telefono
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| "El teléfono es obligatorio".to_string())?;
    let contacto = datos.contacto.as_deref().map(str::trim).filter(|c| !c.is_empty());
    let email = datos.email.as_deref().map(str::trim).filter(|e| !e.is_empty());
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE Proveedores SET Proveedor = ?1, Contacto = ?2, Telefono = ?3, Email = ?4, Activo = ?5
         WHERE ID_Proveedor = ?6",
        params![proveedor, contacto, telefono, email, datos.activo as i64, id],
    )
    .map_err(|e| e.to_string())?;
    bitacora(&pool, &sesion, "Edición", "Proveedor", Some(id), proveedor);
    Ok(())
}

/// Registra una compra: cabecera + detalle, suma al inventario, deja kardex
/// 'Entrada' y (opcional) actualiza el PrecioCosto de cada producto.
#[tauri::command]
pub fn registrar_compra(
    pool: State<Db>,
    sesion: State<Sesion>,
    compra: NuevaCompra,
) -> Result<CompraResultado, String> {
    exigir_admin(&sesion)?;
    if compra.items.is_empty() {
        return Err("La compra no tiene productos".into());
    }

    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let total: f64 = compra
        .items
        .iter()
        .map(|i| i.cantidad * i.costo_unitario)
        .sum();

    // Candado anti-duplicado (evita registrar la misma factura 2 veces):
    //   - con folio: rechaza si ya existe ese folio del mismo proveedor.
    //   - sin folio: heurística (mismo proveedor, total e ítems en la última hora).
    if let Some(folio) = compra.folio.as_deref().map(str::trim).filter(|f| !f.is_empty()) {
        let ya: bool = tx
            .query_row(
                "SELECT 1 FROM Compras
                 WHERE Folio = ?1 AND COALESCE(ID_Proveedor, -1) = COALESCE(?2, -1) LIMIT 1",
                params![folio, compra.id_proveedor],
                |_| Ok(true),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or(false);
        if ya {
            return Err(format!(
                "Ya registraste una compra con el folio '{folio}'. ¿La estás capturando dos veces?"
            ));
        }
    } else {
        // Sin folio/proveedor (compras informales: Costco, Sam's, vendedor de la
        // calle). Se detecta una compra idéntica reciente por su CONTENIDO: mismo
        // proveedor (o ninguno), mismo total, misma cantidad de ítems y, además,
        // los MISMOS productos y cantidades. Ventana amplia (3 días) porque sin
        // folio es la única forma de cuidar al usuario.
        let n_items = compra.items.len() as i64;
        // "Huella" del contenido: lista ordenada de producto:cantidad.
        let mut firma: Vec<String> = compra
            .items
            .iter()
            .map(|i| format!("{}:{}", i.id_producto, i.cantidad))
            .collect();
        firma.sort();
        let firma = firma.join(",");

        let candidatas: Vec<i64> = {
            let mut stmt = tx
                .prepare(
                    "SELECT c.ID_Compra FROM Compras c
                     WHERE COALESCE(c.ID_Proveedor, -1) = COALESCE(?1, -1)
                       AND ABS(c.Total - ?2) < 0.01
                       AND c.FechaCompra >= datetime('now', '-3 days')
                       AND (SELECT COUNT(*) FROM Detalle_Compras d WHERE d.ID_Compra = c.ID_Compra) = ?3",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![compra.id_proveedor, total, n_items], |r| r.get(0))
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<_, _>>().map_err(|e| e.to_string())?
        };
        for id_compra in candidatas {
            let mut otra: Vec<String> = {
                let mut stmt = tx
                    .prepare(
                        "SELECT ID_Producto, Cantidad FROM Detalle_Compras WHERE ID_Compra = ?1",
                    )
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map([id_compra], |r| {
                        Ok(format!("{}:{}", r.get::<_, i64>(0)?, r.get::<_, f64>(1)?))
                    })
                    .map_err(|e| e.to_string())?;
                rows.collect::<Result<_, _>>().map_err(|e| e.to_string())?
            };
            otra.sort();
            if otra.join(",") == firma {
                return Err(
                    "Esta compra es idéntica a una que registraste en los últimos días \
                     (mismos productos, cantidades y total). Si de verdad es OTRA compra, \
                     cambia algo (una cantidad) o ponle un folio para diferenciarla."
                        .into(),
                );
            }
        }
    }

    tx.execute(
        "INSERT INTO Compras (ID_Proveedor, ID_Usuario, Folio, Total) VALUES (?1, ?2, ?3, ?4)",
        params![compra.id_proveedor, compra.id_usuario, compra.folio, total],
    )
    .map_err(|e| e.to_string())?;
    let id_compra = tx.last_insert_rowid();

    for it in &compra.items {
        tx.execute(
            "INSERT INTO Detalle_Compras (ID_Compra, ID_Producto, Cantidad, CostoUnitario)
             VALUES (?1, ?2, ?3, ?4)",
            params![id_compra, it.id_producto, it.cantidad, it.costo_unitario],
        )
        .map_err(|e| e.to_string())?;

        let maneja: i64 = tx
            .query_row(
                "SELECT ManejaCaducidad FROM Productos WHERE ID_Producto = ?1",
                [it.id_producto],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or(0);

        if maneja != 0 {
            // Stock por lotes: cada compra ingresa un lote con su caducidad.
            agregar_lote(
                &tx,
                it.id_producto,
                it.lote.as_deref(),
                it.caducidad.as_deref(),
                it.cantidad,
            )?;
        } else {
            tx.execute(
                "INSERT INTO Inventario (ID_Producto, Cantidad) VALUES (?1, 0)
                 ON CONFLICT(ID_Producto) DO NOTHING",
                params![it.id_producto],
            )
            .map_err(|e| e.to_string())?;

            tx.execute(
                "UPDATE Inventario
                 SET Cantidad = Cantidad + ?1, FechaModificacion = CURRENT_TIMESTAMP
                 WHERE ID_Producto = ?2",
                params![it.cantidad, it.id_producto],
            )
            .map_err(|e| e.to_string())?;
        }

        // Capa de costo PEPS de esta compra (con el costo del proveedor).
        agregar_capa(&tx, it.id_producto, it.cantidad, it.costo_unitario)?;

        tx.execute(
            "INSERT INTO MovimientosInventario (ID_Producto, Tipo, Cantidad, Motivo, ID_Usuario, CostoUnitario)
             VALUES (?1, 'Entrada', ?2, 'Compra', ?3, ?4)",
            params![it.id_producto, it.cantidad, compra.id_usuario, it.costo_unitario],
        )
        .map_err(|e| e.to_string())?;

        // Con PEPS el costo real de venta sale de las capas; PrecioCosto es solo el
        // "último costo conocido" de referencia (para margen/reportes), así que se
        // refresca SIEMPRE con el costo de esta compra. No requiere preguntar.
        tx.execute(
            "UPDATE Productos SET PrecioCosto = ?1 WHERE ID_Producto = ?2",
            params![it.costo_unitario, it.id_producto],
        )
        .map_err(|e| e.to_string())?;
        // Valúa el stock que esté SIN costo (capas en $0, p. ej. de una importación
        // sin PRECIOCOSTO) a este costo. No toca capas ya costeadas.
        tx.execute(
            "UPDATE CapasCosto SET CostoUnitario = ?1
             WHERE ID_Producto = ?2 AND CostoUnitario <= 0",
            params![it.costo_unitario, it.id_producto],
        )
        .map_err(|e| e.to_string())?;

        // Opcional: si la compra trae un precio de venta, también se actualiza.
        if let Some(pv) = it.precio_venta.filter(|p| *p > 0.0) {
            tx.execute(
                "UPDATE Productos SET PrecioUnitario = ?1 WHERE ID_Producto = ?2",
                params![pv, it.id_producto],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    bitacora(
        &pool,
        &sesion,
        "Compra",
        "Inventario",
        Some(id_compra),
        &format!("Compra · {total:.2}"),
    );
    Ok(CompraResultado { id_compra, total })
}

// =====================================================================
//  CLIENTES y FIADO
// =====================================================================

#[tauri::command]
pub fn listar_clientes(
    pool: State<Db>,
    incluir_inactivos: Option<bool>,
) -> Result<Vec<Cliente>, String> {
    let todos = incluir_inactivos.unwrap_or(false) as i64;
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT ID_Cliente, Nombre, Telefono, Email, SaldoFiado, Activo
             FROM Clientes WHERE (?1 = 1 OR Activo = 1) ORDER BY Nombre",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([todos], |r| {
            Ok(Cliente {
                id_cliente: r.get(0)?,
                nombre: r.get(1)?,
                telefono: r.get(2)?,
                email: r.get(3)?,
                saldo_fiado: r.get(4)?,
                activo: r.get::<_, i64>(5)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn crear_cliente(
    pool: State<Db>,
    sesion: State<Sesion>,
    datos: NuevoCliente,
) -> Result<i64, String> {
    usuario_actual(&sesion)?; // cajero o admin (POST permitido al cajero)
    let nombre = datos.nombre.trim();
    if nombre.is_empty() {
        return Err("El nombre del cliente es obligatorio".into());
    }
    let telefono = datos
        .telefono
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| "El teléfono es obligatorio".to_string())?;
    let email = datos.email.as_deref().map(str::trim).filter(|e| !e.is_empty());
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO Clientes (Nombre, Telefono, Email) VALUES (?1, ?2, ?3)",
        params![nombre, telefono, email],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    bitacora(&pool, &sesion, "Alta", "Cliente", Some(id), nombre);
    Ok(id)
}

#[tauri::command]
pub fn actualizar_cliente(
    pool: State<Db>,
    sesion: State<Sesion>,
    id: i64,
    datos: EditarCliente,
) -> Result<(), String> {
    let u = usuario_actual(&sesion)?;
    let nombre = datos.nombre.trim();
    if nombre.is_empty() {
        return Err("El nombre del cliente es obligatorio".into());
    }
    let telefono = datos
        .telefono
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| "El teléfono es obligatorio".to_string())?;
    let email = datos.email.as_deref().map(str::trim).filter(|e| !e.is_empty());
    let conn = pool.get().map_err(|e| e.to_string())?;

    if u.rol == "Administrador" {
        // Admin: puede cambiar todo, incluido activar/desactivar.
        conn.execute(
            "UPDATE Clientes SET Nombre = ?1, Telefono = ?2, Email = ?3, Activo = ?4 WHERE ID_Cliente = ?5",
            params![nombre, telefono, email, datos.activo as i64, id],
        )
        .map_err(|e| e.to_string())?;
    } else {
        // Cajero: solo información personal (nombre/teléfono/email), NO el estado activo.
        conn.execute(
            "UPDATE Clientes SET Nombre = ?1, Telefono = ?2, Email = ?3 WHERE ID_Cliente = ?4",
            params![nombre, telefono, email, id],
        )
        .map_err(|e| e.to_string())?;
    }
    bitacora(&pool, &sesion, "Edición", "Cliente", Some(id), nombre);
    Ok(())
}

/// Estado de cuenta del cliente: cargos (ventas a fiado) y abonos, en orden.
#[tauri::command]
pub fn movimientos_cliente(
    pool: State<Db>,
    id_cliente: i64,
) -> Result<Vec<MovimientoCliente>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT fecha, tipo, referencia, monto FROM (
                SELECT v.FechaVenta AS fecha, 'Cargo' AS tipo, v.Folio AS referencia, v.Total AS monto
                FROM Ventas v
                JOIN MetodosPago m ON m.ID_MetodoPago = v.ID_MetodoPago
                WHERE v.ID_Cliente = ?1 AND m.MetodoPago = 'Fiado' AND v.Estatus = 'Completada'
                UNION ALL
                SELECT a.Fecha, 'Abono', NULL, a.Monto FROM Abonos a WHERE a.ID_Cliente = ?1
             ) ORDER BY fecha",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([id_cliente], |r| {
            Ok(MovimientoCliente {
                fecha: r.get(0)?,
                tipo: r.get(1)?,
                referencia: r.get(2)?,
                monto: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// Registra un abono (pago) de un cliente a su fiado. Baja el SaldoFiado y
/// devuelve el saldo nuevo.
#[tauri::command]
pub fn registrar_abono(
    pool: State<Db>,
    sesion: State<Sesion>,
    id_cliente: i64,
    monto: f64,
    id_usuario: Option<i64>,
    id_venta: Option<i64>,
) -> Result<f64, String> {
    usuario_actual(&sesion)?; // cajero o admin
    if monto <= 0.0 {
        return Err("El abono debe ser mayor a cero".into());
    }

    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "INSERT INTO Abonos (ID_Cliente, ID_Venta, ID_Usuario, Monto) VALUES (?1, ?2, ?3, ?4)",
        params![id_cliente, id_venta, id_usuario, monto],
    )
    .map_err(|e| e.to_string())?;

    tx.execute(
        "UPDATE Clientes SET SaldoFiado = SaldoFiado - ?1 WHERE ID_Cliente = ?2",
        params![monto, id_cliente],
    )
    .map_err(|e| e.to_string())?;

    let saldo: f64 = tx
        .query_row(
            "SELECT SaldoFiado FROM Clientes WHERE ID_Cliente = ?1",
            [id_cliente],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    bitacora(
        &pool,
        &sesion,
        "Abono",
        "Cliente",
        Some(id_cliente),
        &format!("Abono de {:.2}", monto),
    );
    Ok(saldo)
}
