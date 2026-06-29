// =====================================================================
//  models.rs  -  Estructuras que cruzan entre Rust y el front (Svelte).
//  Serde usa camelCase hacia el front (idiomático en JS/TS).
// =====================================================================

use serde::{Deserialize, Serialize};

// ---------- Catálogo ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Categoria {
    pub id_categoria: i64,
    pub categoria: String,
    pub descripcion: Option<String>,
    pub activo: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetodoPago {
    pub id_metodo_pago: i64,
    pub metodo_pago: String,
    pub requiere_referencia: bool,
    pub activo: bool,
}

/// Par llave-valor de la tabla Configuracion (datos del negocio).
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigItem {
    pub clave: String,
    pub valor: String,
}

// ---------- Importación de catálogos ----------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilaProductoImport {
    pub producto: String,
    pub codigo_barras: Option<String>,
    pub precio_venta: f64,
    pub precio_costo: Option<f64>,
    pub existencia: Option<f64>,
    pub categoria: Option<String>,
    #[serde(default)]
    pub unidad: Option<String>,
    /// Se vende a granel por peso/volumen (kg, g, lt, ml) en lugar de por pieza.
    #[serde(default)]
    pub se_vende_peso: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilaClienteImport {
    pub nombre: String,
    pub telefono: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilaCategoriaImport {
    pub categoria: String,
    pub descripcion: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilaProveedorImport {
    pub proveedor: String,
    pub contacto: Option<String>,
    pub telefono: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultadoImport {
    pub insertados: i64,
    pub omitidos: i64,
    pub errores: Vec<String>,
}

/// Una entrada de la bitácora de auditoría.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogBitacora {
    pub id_bitacora: i64,
    pub usuario: Option<String>,
    pub accion: String,
    pub entidad: String,
    pub detalle: Option<String>,
    pub fecha: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnidadMedida {
    pub id_unidad_medida: i64,
    pub unidad_medida: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Moneda {
    pub id_moneda: i64,
    pub moneda: String,
    pub codigo: String,
    pub simbolo: Option<String>,
    pub es_principal: bool,
    pub activo: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Denominacion {
    pub id_denominacion: i64,
    pub id_moneda: i64,
    pub valor: f64,
    pub tipo: Option<String>,
    pub activo: bool,
}

/// Valor por defecto del tipo de artículo cuando el front no lo manda
/// (compatibilidad con llamadas antiguas e importaciones).
fn tipo_predeterminado() -> String {
    "Producto".to_string()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Producto {
    pub id_producto: i64,
    pub producto: String,
    pub codigo_barras: Option<String>,
    pub precio_unitario: f64,
    pub precio_costo: f64,
    pub tipo: String,
    pub maneja_caducidad: bool,
    pub se_vende_peso: bool,
    pub stock_minimo: f64,
    pub id_unidad_medida: Option<i64>,
    pub id_categoria: Option<i64>,
    pub categoria: Option<String>,
    pub existencia: f64,
    pub activo: bool,
    pub unidad: Option<String>,
}

/// Datos para editar un producto (no incluye existencia; eso va por ajuste).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditarProducto {
    pub producto: String,
    pub codigo_barras: Option<String>,
    pub precio_unitario: f64,
    pub precio_costo: f64,
    #[serde(default = "tipo_predeterminado")]
    pub tipo: String,
    #[serde(default)]
    pub maneja_caducidad: bool,
    #[serde(default)]
    pub se_vende_peso: bool,
    #[serde(default)]
    pub stock_minimo: f64,
    pub id_unidad_medida: Option<i64>,
    pub id_categoria: Option<i64>,
    pub activo: bool,
}

/// Datos para dar de alta un producto. existencia_inicial es opcional.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuevoProducto {
    pub producto: String,
    pub codigo_barras: Option<String>,
    pub precio_unitario: f64,
    pub precio_costo: f64,
    #[serde(default = "tipo_predeterminado")]
    pub tipo: String,
    #[serde(default)]
    pub maneja_caducidad: bool,
    #[serde(default)]
    pub se_vende_peso: bool,
    #[serde(default)]
    pub stock_minimo: f64,
    pub id_unidad_medida: Option<i64>,
    pub id_categoria: Option<i64>,
    #[serde(default)]
    pub existencia_inicial: f64,
    /// Para productos con caducidad: lote y caducidad de la existencia inicial.
    #[serde(default)]
    pub lote_inicial: Option<String>,
    #[serde(default)]
    pub caducidad_inicial: Option<String>,
}

// ---------- Lotes / caducidad ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lote {
    pub id_lote: i64,
    pub id_producto: i64,
    pub lote: Option<String>,
    pub caducidad: Option<String>,
    pub cantidad: f64,
}

/// Lote con info del producto y días restantes, para la pantalla de alertas.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoteVencimiento {
    pub id_lote: i64,
    pub id_producto: i64,
    pub producto: String,
    pub lote: Option<String>,
    pub caducidad: Option<String>,
    pub cantidad: f64,
    /// Días para caducar (negativo = ya venció). NULL si el lote no tiene caducidad.
    pub dias_restantes: Option<i64>,
}

// ---------- Reporte de movimientos (entradas/salidas) ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovimientoResumen {
    pub producto: String,
    pub comprado: f64,
    pub vendido: f64,
    pub merma: f64,
    pub entradas: f64,
    pub salidas: f64,
}

/// Una línea del historial (kardex) de un producto: alta, venta, merma, etc.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovimientoDetalle {
    pub fecha: String,
    pub tipo: String,            // 'Entrada' | 'Venta' | 'Merma' | 'Ajuste'
    pub cantidad: f64,           // positivo = entró, negativo = salió
    pub motivo: Option<String>,  // "Compra", "Venta", "Caducidad", "Robo", etc.
    pub usuario: Option<String>,
}

// ---------- Reportes de costeo PEPS (utilidad e inventario valorizado) ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UtilidadProducto {
    pub producto: String,
    pub vendido: f64, // unidades
    pub ventas: f64,  // ingreso
    pub costo: f64,   // costo de ventas (COGS, PEPS)
    pub utilidad: f64,
    pub margen: f64, // % sobre ventas
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventarioValorizado {
    pub producto: String,
    pub existencia: f64,
    pub costo_unitario: f64, // costo promedio de las capas restantes (PEPS)
    pub valor: f64,          // existencia valuada a PEPS
}

/// Una línea del kardex valorizado (tarjeta de almacén) de un producto.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KardexLinea {
    pub fecha: String,
    pub concepto: String,     // motivo (o tipo): Compra, Venta, Caducidad, etc.
    pub cantidad: f64,        // con signo: positivo entra, negativo sale
    pub costo_unitario: f64,
    pub saldo_cantidad: f64,  // existencia acumulada
    pub saldo_valor: f64,     // valor acumulado (PEPS)
}

// ---------- Ventas ----------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemVenta {
    pub id_producto: i64,
    pub cantidad: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuevaVenta {
    pub id_usuario: i64,
    pub id_metodo_pago: i64,
    pub id_corte: Option<i64>,
    pub id_cliente: Option<i64>,
    pub referencia_pago: Option<String>,
    pub pago_con: Option<f64>,
    pub items: Vec<ItemVenta>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VentaResultado {
    pub id_venta: i64,
    pub folio: String,
    pub total: f64,
    pub cambio: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VentaResumen {
    pub id_venta: i64,
    pub folio: Option<String>,
    pub fecha_venta: String,
    pub total: f64,
    pub estatus: String,
    pub metodo_pago: String,
    pub usuario: String,
    pub cliente: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetalleVentaLinea {
    pub producto: String,
    pub cantidad: f64,
    pub precio: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReporteMetodo {
    pub metodo_pago: String,
    pub total: f64,
    pub tickets: i64,
}

/// Cierre de ventas: total y desglose por método en un rango de fechas.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReporteVentas {
    pub total: f64,
    pub tickets: i64,
    pub canceladas: i64,
    pub metodos: Vec<ReporteMetodo>,
}

// ---------- Usuarios ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usuario {
    pub id_usuario: i64,
    pub nombre: String,
    pub usuario: String,
    pub rol: String,
    pub activo: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuevoUsuario {
    pub nombre: String,
    pub usuario: String,
    pub contrasena: String,
    pub rol: String, // 'Administrador' o 'Cajero'
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditarUsuario {
    pub nombre: String,
    pub usuario: String,
    pub rol: String,
    pub activo: bool,
}

// ---------- Caja (cortes) ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Corte {
    pub id_corte: i64,
    pub id_usuario: i64,
    pub fecha_apertura: String,
    pub monto_inicial: f64,
    pub estatus: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumenCorte {
    pub id_corte: i64,
    pub monto_inicial: f64,
    pub ventas_efectivo: f64,
    pub ingresos: f64,
    pub retiros: f64,
    pub monto_esperado: f64,
    pub monto_contado: f64,
    pub diferencia: f64,
}

/// Corte histórico para reportes.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorteHistorico {
    pub id_corte: i64,
    pub usuario: String,
    pub fecha_apertura: String,
    pub fecha_cierre: Option<String>,
    pub monto_inicial: f64,
    pub monto_esperado: Option<f64>,
    pub monto_contado: Option<f64>,
    pub diferencia: Option<f64>,
    pub estatus: String,
}

/// Totales en vivo del corte (sin cerrarlo todavía).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumenParcialCorte {
    pub id_corte: i64,
    pub monto_inicial: f64,
    pub ventas_efectivo: f64,
    pub ingresos: f64,
    pub retiros: f64,
    pub monto_esperado: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovimientoCaja {
    pub id_movimiento: i64,
    pub tipo: String,
    pub monto: f64,
    pub concepto: Option<String>,
    pub fecha: String,
}

// ---------- Proveedores y compras ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proveedor {
    pub id_proveedor: i64,
    pub proveedor: String,
    pub contacto: Option<String>,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub activo: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuevoProveedor {
    pub proveedor: String,
    pub contacto: Option<String>,
    pub telefono: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditarProveedor {
    pub proveedor: String,
    pub contacto: Option<String>,
    pub telefono: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    pub activo: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemCompra {
    pub id_producto: i64,
    pub cantidad: f64,
    pub costo_unitario: f64,
    /// Opcional: si viene, actualiza también el precio de venta del producto.
    #[serde(default)]
    pub precio_venta: Option<f64>,
    /// Solo para productos con caducidad: lote y fecha de caducidad recibidos.
    #[serde(default)]
    pub lote: Option<String>,
    #[serde(default)]
    pub caducidad: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuevaCompra {
    pub id_proveedor: Option<i64>,
    pub id_usuario: Option<i64>,
    pub folio: Option<String>,
    pub items: Vec<ItemCompra>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompraResultado {
    pub id_compra: i64,
    pub total: f64,
}

// ---------- Clientes (fiado) ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cliente {
    pub id_cliente: i64,
    pub nombre: String,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub saldo_fiado: f64,
    pub activo: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuevoCliente {
    pub nombre: String,
    pub telefono: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditarCliente {
    pub nombre: String,
    pub telefono: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    pub activo: bool,
}

/// Una línea del estado de cuenta de un cliente: cargo (venta a fiado) o abono.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovimientoCliente {
    pub fecha: String,
    pub tipo: String, // 'Cargo' | 'Abono'
    pub referencia: Option<String>,
    pub monto: f64,
}
