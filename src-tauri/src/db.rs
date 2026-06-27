// =====================================================================
//  db.rs  -  Conexión y manejo de la base de datos local (SQLite)
//
//  - Pool de conexiones (r2d2) compartido como estado de Tauri.
//  - Crea el esquema (posdb.sql) la primera vez que se abre la app.
//  - Helpers para leer/guardar la licencia en la tabla LicenciaLocal.
// =====================================================================

use crate::license::LicensePayload;
use chrono::{DateTime, Utc};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use std::error::Error;
use tauri::{AppHandle, Manager};

/// Tipo corto para el pool. Es lo que se inyecta en los comandos como State<Db>.
pub type Db = Pool<SqliteConnectionManager>;

/// El esquema vive en posdb.sql (raíz del proyecto). Se incrusta en el .exe
/// en tiempo de compilación, así que no hay que distribuir el .sql aparte.
const SCHEMA: &str = include_str!("../../scripts/posdb.sql");

/// Abre (o crea) la base en la carpeta de datos de la app del usuario:
///   Windows -> C:\Users\<user>\AppData\Roaming\com.aquastudio.aquapos\pos.db
pub fn init(app: &AppHandle) -> Result<Db, Box<dyn Error>> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let db_path = dir.join("pos.db");

    // En cada conexión nueva: activar llaves foráneas y modo WAL (mejor
    // concurrencia lectura/escritura para un POS).
    let manager = SqliteConnectionManager::file(&db_path).with_init(|c| {
        c.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
    });
    let pool = Pool::new(manager)?;

    // Si no existe la tabla Productos, asumimos base vacía y corremos el esquema.
    let conn = pool.get()?;
    let ya_existe: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='Productos'",
            [],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);

    if !ya_existe {
        conn.execute_batch(SCHEMA)?;
        sembrar_datos_iniciales(&conn)?;
    }

    // Migración: bitácora de auditoría. IF NOT EXISTS para cubrir bases ya creadas.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS Bitacora (
            ID_Bitacora   INTEGER PRIMARY KEY AUTOINCREMENT,
            ID_Usuario    INTEGER,
            Usuario       TEXT,
            Accion        TEXT NOT NULL,
            Entidad       TEXT NOT NULL,
            ID_Referencia INTEGER,
            Detalle       TEXT,
            Fecha         DATETIME DEFAULT CURRENT_TIMESTAMP
         );
         CREATE INDEX IF NOT EXISTS idx_bitacora_fecha ON Bitacora(Fecha);",
    )?;

    // Migración: tipo de artículo (Producto/Servicio) en bases ya creadas.
    agregar_columna_si_falta(
        &conn,
        "Productos",
        "Tipo",
        "TEXT NOT NULL DEFAULT 'Producto'",
    )?;

    // Migración: caducidad/lote (farmacia). Flag por producto + tabla de lotes.
    agregar_columna_si_falta(&conn, "Productos", "ManejaCaducidad", "INTEGER DEFAULT 0")?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS Lotes (
            ID_Lote       INTEGER PRIMARY KEY AUTOINCREMENT,
            ID_Producto   INTEGER NOT NULL,
            Lote          TEXT,
            Caducidad     TEXT,                 -- 'YYYY-MM-DD' (puede ser NULL)
            Cantidad      REAL NOT NULL DEFAULT 0,
            FechaCreacion DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (ID_Producto) REFERENCES Productos(ID_Producto)
         );
         CREATE INDEX IF NOT EXISTS idx_lotes_producto ON Lotes(ID_Producto);
         CREATE INDEX IF NOT EXISTS idx_lotes_caducidad ON Lotes(Caducidad);",
    )?;

    // Migración: catálogo de monedas y sus denominaciones.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS Monedas (
            ID_Moneda     INTEGER PRIMARY KEY AUTOINCREMENT,
            Moneda        TEXT NOT NULL,
            Codigo        TEXT NOT NULL UNIQUE,
            Simbolo       TEXT,
            EsPrincipal   INTEGER DEFAULT 0,
            Activo        INTEGER DEFAULT 1,
            FechaCreacion DATETIME DEFAULT CURRENT_TIMESTAMP
         );
         CREATE TABLE IF NOT EXISTS Denominaciones (
            ID_Denominacion INTEGER PRIMARY KEY AUTOINCREMENT,
            ID_Moneda       INTEGER NOT NULL,
            Valor           REAL NOT NULL,
            Tipo            TEXT,
            Activo          INTEGER DEFAULT 1,
            FOREIGN KEY (ID_Moneda) REFERENCES Monedas(ID_Moneda)
         );",
    )?;

    // Siembra de Peso Mexicano + sus denominaciones (solo si no hay monedas).
    let hay_monedas: i64 = conn.query_row("SELECT COUNT(*) FROM Monedas", [], |r| r.get(0))?;
    if hay_monedas == 0 {
        conn.execute(
            "INSERT INTO Monedas (Moneda, Codigo, Simbolo, EsPrincipal) VALUES ('Peso Mexicano', 'MXN', '$', 1)",
            [],
        )?;
        let id = conn.last_insert_rowid();
        for v in [1000.0_f64, 500.0, 200.0, 100.0, 50.0, 20.0] {
            conn.execute(
                "INSERT INTO Denominaciones (ID_Moneda, Valor, Tipo) VALUES (?1, ?2, 'Billete')",
                params![id, v],
            )?;
        }
        for v in [10.0_f64, 5.0, 2.0, 1.0, 0.5] {
            conn.execute(
                "INSERT INTO Denominaciones (ID_Moneda, Valor, Tipo) VALUES (?1, ?2, 'Moneda')",
                params![id, v],
            )?;
        }
    }

    Ok(pool)
}

/// Datos mínimos para que el POS sea usable apenas se crea la base:
/// unidades y métodos de pago (las ventas los necesitan).
///
/// Las categorías NO se siembran: el cliente elige el catálogo de categorías
/// según su giro durante la configuración inicial (importándolo desde un CSV).
fn sembrar_datos_iniciales(conn: &rusqlite::Connection) -> Result<(), Box<dyn Error>> {
    conn.execute_batch(
        "INSERT INTO UnidadMedida (UnidadMedida) VALUES ('Pieza'), ('Kilogramo'), ('Gramo'), ('Litro'), ('Mililitro'), ('Caja');

         INSERT INTO MetodosPago (MetodoPago, RequiereReferencia) VALUES
            ('Efectivo', 0),
            ('Tarjeta', 1),
            ('Transferencia', 1),
            ('Fiado', 0);",
    )?;
    Ok(())
}

/// Agrega una columna a una tabla solo si aún no existe (migración idempotente).
/// `tabla` y `columna` son constantes del código (sin entrada del usuario).
fn agregar_columna_si_falta(
    conn: &rusqlite::Connection,
    tabla: &str,
    columna: &str,
    definicion: &str,
) -> Result<(), Box<dyn Error>> {
    let existe: bool = conn
        .query_row(
            &format!("SELECT 1 FROM pragma_table_info('{tabla}') WHERE name = '{columna}'"),
            [],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !existe {
        conn.execute_batch(&format!(
            "ALTER TABLE {tabla} ADD COLUMN {columna} {definicion}"
        ))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------
//  LICENCIA (tabla LicenciaLocal, una sola fila con ID = 1)
// ---------------------------------------------------------------------

/// Fecha de inicio del periodo de prueba. Si no existe, la crea con la fecha
/// actual (primer arranque). Se guarda en Configuracion como llave-valor.
pub fn trial_inicio(pool: &Db) -> Result<DateTime<Utc>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let existente: Option<String> = conn
        .query_row(
            "SELECT Valor FROM Configuracion WHERE Clave = 'trial_inicio'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if let Some(s) = existente {
        return DateTime::parse_from_rfc3339(&s)
            .map(|d| d.with_timezone(&Utc))
            .map_err(|e| e.to_string());
    }

    let ahora = Utc::now();
    conn.execute(
        "INSERT INTO Configuracion (Clave, Valor) VALUES ('trial_inicio', ?1)",
        params![ahora.to_rfc3339()],
    )
    .map_err(|e| e.to_string())?;
    Ok(ahora)
}

/// Lo que necesitamos del registro local para evaluar la licencia.
pub struct LicenciaRow {
    pub clave: Option<String>,
    pub token: Option<String>,
    pub ultimo_chequeo: Option<DateTime<Utc>>,
}

/// Lee la fila de licencia (None si nunca se ha activado).
pub fn leer_licencia(pool: &Db) -> Result<Option<LicenciaRow>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT Clave, TokenFirmado, UltimoChequeoOnline FROM LicenciaLocal WHERE ID = 1",
        [],
        |row| {
            let chequeo: Option<String> = row.get(2)?;
            Ok(LicenciaRow {
                clave: row.get(0)?,
                token: row.get(1)?,
                ultimo_chequeo: chequeo
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|d| d.with_timezone(&Utc)),
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// Guarda (inserta o reemplaza) la licencia tras una activación/revalidación
/// exitosa. Deja UltimoChequeoOnline = ahora.
pub fn guardar_licencia(
    pool: &Db,
    clave: &str,
    token: &str,
    payload: &LicensePayload,
) -> Result<(), String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO LicenciaLocal
            (ID, Clave, TokenFirmado, MachineID, FechaActivacion, FechaExpiracion, UltimoChequeoOnline, Estado)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, 'Activa')
         ON CONFLICT(ID) DO UPDATE SET
            Clave               = excluded.Clave,
            TokenFirmado        = excluded.TokenFirmado,
            MachineID           = excluded.MachineID,
            FechaActivacion     = excluded.FechaActivacion,
            FechaExpiracion     = excluded.FechaExpiracion,
            UltimoChequeoOnline = excluded.UltimoChequeoOnline,
            Estado              = excluded.Estado",
        params![
            clave,
            token,
            payload.machine_id,
            payload.fecha_activacion.to_rfc3339(),
            payload.fecha_expiracion.map(|d| d.to_rfc3339()),
            Utc::now().to_rfc3339(),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
