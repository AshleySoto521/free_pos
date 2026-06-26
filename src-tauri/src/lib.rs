mod commands;
mod db;
mod license;
mod models;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::Sesion::default())
        .setup(|app| {
            // Abre/crea la base de datos local y la deja disponible como estado
            // compartido para todos los comandos (State<Db>).
            let pool = db::init(app.handle())?;
            app.manage(pool);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // --- Licencia ---
            commands::licencia_estado,
            commands::asegurar_trial,
            commands::licencia_activar,
            commands::licencia_revalidar,
            // --- Usuarios / login ---
            commands::login,
            commands::cerrar_sesion,
            commands::sesion_actual,
            commands::crear_usuario,
            commands::actualizar_usuario,
            commands::cambiar_contrasena,
            commands::listar_usuarios,
            commands::listar_bitacora,
            // --- Catálogo ---
            commands::listar_categorias,
            commands::crear_categoria,
            commands::actualizar_categoria,
            commands::listar_unidades,
            commands::crear_unidad,
            commands::actualizar_unidad,
            commands::listar_metodos_pago,
            commands::crear_metodo_pago,
            commands::actualizar_metodo_pago,
            commands::listar_monedas,
            commands::crear_moneda,
            commands::actualizar_moneda,
            commands::listar_denominaciones,
            commands::crear_denominacion,
            commands::eliminar_denominacion,
            commands::listar_config,
            commands::guardar_config,
            commands::listar_productos,
            commands::buscar_producto_por_codigo,
            commands::crear_producto,
            commands::actualizar_producto,
            commands::ajustar_inventario,
            commands::listar_lotes,
            commands::lotes_por_vencer,
            commands::dar_baja_lote,
            commands::importar_productos,
            commands::importar_clientes,
            commands::importar_categorias,
            commands::importar_proveedores,
            // --- Ventas ---
            commands::registrar_venta,
            commands::cancelar_venta,
            commands::listar_ventas,
            commands::detalle_venta,
            commands::reporte_ventas,
            // --- Caja (cortes) ---
            commands::abrir_corte,
            commands::corte_abierto,
            commands::resumen_corte,
            commands::listar_movimientos_caja,
            commands::registrar_movimiento_caja,
            commands::cerrar_corte,
            commands::listar_cortes,
            // --- Proveedores / compras ---
            commands::listar_proveedores,
            commands::crear_proveedor,
            commands::actualizar_proveedor,
            commands::registrar_compra,
            // --- Clientes / fiado ---
            commands::listar_clientes,
            commands::crear_cliente,
            commands::actualizar_cliente,
            commands::movimientos_cliente,
            commands::registrar_abono,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar Free POS");
}
