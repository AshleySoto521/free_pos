// =====================================================================
//  api.ts  -  Wrapper tipado sobre los comandos del backend de Tauri.
//
//  Cada función llama a invoke() con el nombre exacto del comando Rust.
//  Los argumentos van en camelCase (Tauri v2 los convierte a snake_case).
//  Importa desde aquí en los componentes:  import { api } from '$lib/api';
//
//  Nota: invoke() solo funciona dentro de la ventana de Tauri
//  (`pnpm tauri dev`). En `pnpm dev` (navegador puro) lanzará error.
// =====================================================================

import { invoke } from '@tauri-apps/api/core';

// ---------------------------------------------------------------------
//  Tipos (espejo de src-tauri/src/models.rs y license.rs)
// ---------------------------------------------------------------------

export interface LicenseStatus {
	valida: boolean;
	estado: string; // Activa/Expirada/SinActivar/GraciaAgotada/Manipulada/Error
	mensaje: string;
	fechaExpiracion: string | null;
	diasRestantes: number | null;
}

export interface Usuario {
	idUsuario: number;
	nombre: string;
	usuario: string;
	rol: string;
	activo: boolean;
}
export interface NuevoUsuario {
	nombre: string;
	usuario: string;
	contrasena: string;
	rol: string; // 'Administrador' | 'Cajero'
}

export interface LogBitacora {
	idBitacora: number;
	usuario: string | null;
	accion: string;
	entidad: string;
	detalle: string | null;
	fecha: string;
}

export interface FilaProductoImport {
	producto: string;
	codigoBarras?: string | null;
	precioVenta: number;
	precioCosto?: number | null;
	existencia?: number | null;
	categoria?: string | null;
}
export interface FilaClienteImport {
	nombre: string;
	telefono?: string | null;
}
export interface FilaCategoriaImport {
	categoria: string;
	descripcion?: string | null;
}
export interface FilaProveedorImport {
	proveedor: string;
	contacto?: string | null;
	telefono?: string | null;
}
export interface ResultadoImport {
	insertados: number;
	omitidos: number;
	errores: string[];
}
export interface EditarUsuario {
	nombre: string;
	usuario: string;
	rol: string;
	activo: boolean;
}

export interface Categoria {
	idCategoria: number;
	categoria: string;
	descripcion: string | null;
	activo: boolean;
}

export interface MovimientoCliente {
	fecha: string;
	tipo: string; // 'Cargo' | 'Abono'
	referencia: string | null;
	monto: number;
}

export interface MetodoPago {
	idMetodoPago: number;
	metodoPago: string;
	requiereReferencia: boolean;
	activo: boolean;
}

export interface ConfigItem {
	clave: string;
	valor: string;
}

export interface UnidadMedida {
	idUnidadMedida: number;
	unidadMedida: string;
}

export interface Moneda {
	idMoneda: number;
	moneda: string;
	codigo: string;
	simbolo: string | null;
	esPrincipal: boolean;
	activo: boolean;
}

export interface Denominacion {
	idDenominacion: number;
	idMoneda: number;
	valor: number;
	tipo: string | null;
	activo: boolean;
}

export interface Producto {
	idProducto: number;
	producto: string;
	codigoBarras: string | null;
	precioUnitario: number;
	precioCosto: number;
	tipo: string; // 'Producto' | 'Servicio'
	manejaCaducidad: boolean;
	seVendePeso: boolean;
	stockMinimo: number;
	idUnidadMedida: number | null;
	idCategoria: number | null;
	categoria: string | null;
	existencia: number;
	activo: boolean;
}

export interface EditarProducto {
	producto: string;
	codigoBarras?: string | null;
	precioUnitario: number;
	precioCosto: number;
	tipo?: string;
	manejaCaducidad?: boolean;
	seVendePeso?: boolean;
	stockMinimo?: number;
	idUnidadMedida?: number | null;
	idCategoria?: number | null;
	activo: boolean;
}
export interface NuevoProducto {
	producto: string;
	codigoBarras?: string | null;
	precioUnitario: number;
	precioCosto: number;
	tipo?: string;
	manejaCaducidad?: boolean;
	seVendePeso?: boolean;
	stockMinimo?: number;
	idUnidadMedida?: number | null;
	idCategoria?: number | null;
	existenciaInicial?: number;
	loteInicial?: string | null;
	caducidadInicial?: string | null;
}

export interface Lote {
	idLote: number;
	idProducto: number;
	lote: string | null;
	caducidad: string | null;
	cantidad: number;
}
export interface LoteVencimiento {
	idLote: number;
	idProducto: number;
	producto: string;
	lote: string | null;
	caducidad: string | null;
	cantidad: number;
	diasRestantes: number | null;
}

export interface ItemVenta {
	idProducto: number;
	cantidad: number;
}
export interface NuevaVenta {
	idUsuario: number;
	idMetodoPago: number;
	idCorte?: number | null;
	idCliente?: number | null;
	referenciaPago?: string | null;
	pagoCon?: number | null;
	items: ItemVenta[];
}
export interface VentaResultado {
	idVenta: number;
	folio: string;
	total: number;
	cambio: number | null;
}

export interface VentaResumen {
	idVenta: number;
	folio: string | null;
	fechaVenta: string;
	total: number;
	estatus: string;
	metodoPago: string;
	usuario: string;
	cliente: string | null;
}

export interface DetalleVentaLinea {
	producto: string;
	cantidad: number;
	precio: number;
}

export interface ReporteMetodo {
	metodoPago: string;
	total: number;
	tickets: number;
}

export interface ReporteVentas {
	total: number;
	tickets: number;
	canceladas: number;
	metodos: ReporteMetodo[];
}

export interface Corte {
	idCorte: number;
	idUsuario: number;
	fechaApertura: string;
	montoInicial: number;
	estatus: string;
}
export interface ResumenCorte {
	idCorte: number;
	montoInicial: number;
	ventasEfectivo: number;
	ingresos: number;
	retiros: number;
	montoEsperado: number;
	montoContado: number;
	diferencia: number;
}

export interface ResumenParcialCorte {
	idCorte: number;
	montoInicial: number;
	ventasEfectivo: number;
	ingresos: number;
	retiros: number;
	montoEsperado: number;
}

export interface CorteHistorico {
	idCorte: number;
	usuario: string;
	fechaApertura: string;
	fechaCierre: string | null;
	montoInicial: number;
	montoEsperado: number | null;
	montoContado: number | null;
	diferencia: number | null;
	estatus: string;
}

export interface MovimientoCaja {
	idMovimiento: number;
	tipo: string;
	monto: number;
	concepto: string | null;
	fecha: string;
}

export interface Proveedor {
	idProveedor: number;
	proveedor: string;
	contacto: string | null;
	telefono: string | null;
	activo: boolean;
}
export interface NuevoProveedor {
	proveedor: string;
	contacto?: string | null;
	telefono?: string | null;
}
export interface EditarProveedor {
	proveedor: string;
	contacto?: string | null;
	telefono?: string | null;
	activo: boolean;
}

export interface ItemCompra {
	idProducto: number;
	cantidad: number;
	costoUnitario: number;
	lote?: string | null;
	caducidad?: string | null;
}
export interface NuevaCompra {
	idProveedor?: number | null;
	idUsuario?: number | null;
	folio?: string | null;
	actualizarCosto?: boolean;
	items: ItemCompra[];
}
export interface CompraResultado {
	idCompra: number;
	total: number;
}

export interface Cliente {
	idCliente: number;
	nombre: string;
	telefono: string | null;
	saldoFiado: number;
	activo: boolean;
}
export interface NuevoCliente {
	nombre: string;
	telefono?: string | null;
}
export interface EditarCliente {
	nombre: string;
	telefono?: string | null;
	activo: boolean;
}

// ---------------------------------------------------------------------
//  API
// ---------------------------------------------------------------------

export const api = {
	// ----- Licencia -----
	licenciaEstado: () => invoke<LicenseStatus>('licencia_estado'),
	asegurarTrial: () => invoke<LicenseStatus>('asegurar_trial'),
	licenciaActivar: (clave: string) => invoke<LicenseStatus>('licencia_activar', { clave }),
	licenciaRevalidar: () => invoke<LicenseStatus>('licencia_revalidar'),

	// ----- Usuarios / login -----
	login: (usuario: string, contrasena: string) =>
		invoke<Usuario>('login', { usuario, contrasena }),
	cerrarSesion: () => invoke<void>('cerrar_sesion'),
	sesionActual: () => invoke<Usuario | null>('sesion_actual'),
	crearUsuario: (datos: NuevoUsuario) => invoke<number>('crear_usuario', { datos }),
	actualizarUsuario: (id: number, datos: EditarUsuario) =>
		invoke<void>('actualizar_usuario', { id, datos }),
	cambiarContrasena: (id: number, nueva: string) =>
		invoke<void>('cambiar_contrasena', { id, nueva }),
	listarUsuarios: () => invoke<Usuario[]>('listar_usuarios'),
	listarBitacora: (desde: string, hasta: string) =>
		invoke<LogBitacora[]>('listar_bitacora', { desde, hasta }),

	// ----- Catálogo -----
	listarCategorias: (incluirInactivos?: boolean) =>
		invoke<Categoria[]>('listar_categorias', { incluirInactivos }),
	crearCategoria: (categoria: string, descripcion?: string | null) =>
		invoke<number>('crear_categoria', { categoria, descripcion }),
	actualizarCategoria: (
		id: number,
		categoria: string,
		descripcion: string | null,
		activo: boolean
	) => invoke<void>('actualizar_categoria', { id, categoria, descripcion, activo }),
	listarUnidades: () => invoke<UnidadMedida[]>('listar_unidades'),
	crearUnidad: (nombre: string) => invoke<number>('crear_unidad', { nombre }),
	actualizarUnidad: (id: number, nombre: string) =>
		invoke<void>('actualizar_unidad', { id, nombre }),
	listarMetodosPago: (incluirInactivos?: boolean) =>
		invoke<MetodoPago[]>('listar_metodos_pago', { incluirInactivos }),
	crearMetodoPago: (metodoPago: string, requiereReferencia: boolean) =>
		invoke<number>('crear_metodo_pago', { metodoPago, requiereReferencia }),
	actualizarMetodoPago: (
		id: number,
		metodoPago: string,
		requiereReferencia: boolean,
		activo: boolean
	) => invoke<void>('actualizar_metodo_pago', { id, metodoPago, requiereReferencia, activo }),
	listarMonedas: (incluirInactivos?: boolean) =>
		invoke<Moneda[]>('listar_monedas', { incluirInactivos }),
	crearMoneda: (moneda: string, codigo: string, simbolo: string | null, esPrincipal: boolean) =>
		invoke<number>('crear_moneda', { moneda, codigo, simbolo, esPrincipal }),
	actualizarMoneda: (
		id: number,
		moneda: string,
		codigo: string,
		simbolo: string | null,
		esPrincipal: boolean,
		activo: boolean
	) => invoke<void>('actualizar_moneda', { id, moneda, codigo, simbolo, esPrincipal, activo }),
	listarDenominaciones: (idMoneda: number) =>
		invoke<Denominacion[]>('listar_denominaciones', { idMoneda }),
	crearDenominacion: (idMoneda: number, valor: number, tipo: string | null) =>
		invoke<number>('crear_denominacion', { idMoneda, valor, tipo }),
	eliminarDenominacion: (id: number) => invoke<void>('eliminar_denominacion', { id }),
	listarConfig: () => invoke<ConfigItem[]>('listar_config'),
	guardarConfig: (items: ConfigItem[]) => invoke<void>('guardar_config', { items }),
	listarProductos: (incluirInactivos?: boolean) =>
		invoke<Producto[]>('listar_productos', { incluirInactivos }),
	buscarProductoPorCodigo: (codigo: string) =>
		invoke<Producto | null>('buscar_producto_por_codigo', { codigo }),
	crearProducto: (datos: NuevoProducto) => invoke<number>('crear_producto', { datos }),
	actualizarProducto: (id: number, datos: EditarProducto) =>
		invoke<void>('actualizar_producto', { id, datos }),
	importarProductos: (filas: FilaProductoImport[]) =>
		invoke<ResultadoImport>('importar_productos', { filas }),
	importarClientes: (filas: FilaClienteImport[]) =>
		invoke<ResultadoImport>('importar_clientes', { filas }),
	importarCategorias: (filas: FilaCategoriaImport[]) =>
		invoke<ResultadoImport>('importar_categorias', { filas }),
	importarProveedores: (filas: FilaProveedorImport[]) =>
		invoke<ResultadoImport>('importar_proveedores', { filas }),
	ajustarInventario: (
		idProducto: number,
		tipo: 'Entrada' | 'Merma' | 'Ajuste',
		cantidad: number,
		motivo?: string | null,
		idUsuario?: number | null
	) => invoke<number>('ajustar_inventario', { idProducto, tipo, cantidad, motivo, idUsuario }),

	// ----- Lotes / caducidad -----
	listarLotes: (idProducto: number) => invoke<Lote[]>('listar_lotes', { idProducto }),
	lotesPorVencer: (dias: number) => invoke<LoteVencimiento[]>('lotes_por_vencer', { dias }),
	darBajaLote: (idLote: number, motivo?: string | null, idUsuario?: number | null) =>
		invoke<void>('dar_baja_lote', { idLote, motivo, idUsuario }),

	// ----- Ventas -----
	registrarVenta: (venta: NuevaVenta) => invoke<VentaResultado>('registrar_venta', { venta }),
	cancelarVenta: (
		idVenta: number,
		idUsuario: number,
		disposicion: 'regresar' | 'merma' | 'descartar'
	) => invoke<void>('cancelar_venta', { idVenta, idUsuario, disposicion }),
	listarVentas: (desde: string, hasta: string) =>
		invoke<VentaResumen[]>('listar_ventas', { desde, hasta }),
	detalleVenta: (idVenta: number) =>
		invoke<DetalleVentaLinea[]>('detalle_venta', { idVenta }),
	reporteVentas: (desde: string, hasta: string) =>
		invoke<ReporteVentas>('reporte_ventas', { desde, hasta }),

	// ----- Caja (cortes) -----
	abrirCorte: (idUsuario: number, montoInicial: number) =>
		invoke<number>('abrir_corte', { idUsuario, montoInicial }),
	corteAbierto: (idUsuario: number) => invoke<Corte | null>('corte_abierto', { idUsuario }),
	resumenCorte: (idCorte: number) => invoke<ResumenParcialCorte>('resumen_corte', { idCorte }),
	listarMovimientosCaja: (idCorte: number) =>
		invoke<MovimientoCaja[]>('listar_movimientos_caja', { idCorte }),
	registrarMovimientoCaja: (
		idCorte: number,
		tipo: 'Ingreso' | 'Retiro',
		monto: number,
		concepto?: string | null
	) => invoke<number>('registrar_movimiento_caja', { idCorte, tipo, monto, concepto }),
	cerrarCorte: (idCorte: number, montoContado: number) =>
		invoke<ResumenCorte>('cerrar_corte', { idCorte, montoContado }),
	listarCortes: (desde: string, hasta: string) =>
		invoke<CorteHistorico[]>('listar_cortes', { desde, hasta }),

	// ----- Proveedores / compras -----
	listarProveedores: (incluirInactivos?: boolean) =>
		invoke<Proveedor[]>('listar_proveedores', { incluirInactivos }),
	crearProveedor: (datos: NuevoProveedor) => invoke<number>('crear_proveedor', { datos }),
	actualizarProveedor: (id: number, datos: EditarProveedor) =>
		invoke<void>('actualizar_proveedor', { id, datos }),
	registrarCompra: (compra: NuevaCompra) =>
		invoke<CompraResultado>('registrar_compra', { compra }),

	// ----- Clientes / fiado -----
	listarClientes: (incluirInactivos?: boolean) =>
		invoke<Cliente[]>('listar_clientes', { incluirInactivos }),
	crearCliente: (datos: NuevoCliente) => invoke<number>('crear_cliente', { datos }),
	actualizarCliente: (id: number, datos: EditarCliente) =>
		invoke<void>('actualizar_cliente', { id, datos }),
	movimientosCliente: (idCliente: number) =>
		invoke<MovimientoCliente[]>('movimientos_cliente', { idCliente }),
	registrarAbono: (
		idCliente: number,
		monto: number,
		idUsuario?: number | null,
		idVenta?: number | null
	) => invoke<number>('registrar_abono', { idCliente, monto, idUsuario, idVenta })
};
