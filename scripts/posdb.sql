-- =====================================================================
--  POS MISCELÁNEA "Tienda de la esquina" - Esquema de base de datos
--  SQLite
-- =====================================================================

PRAGMA foreign_keys = ON;

-- =====================================================================
--  CATÁLOGOS BÁSICOS
-- =====================================================================

-- 1. UNIDADES DE MEDIDA (Pza, Kg, Litro)
CREATE TABLE "UnidadMedida" (
  "ID_UnidadMedida" INTEGER PRIMARY KEY AUTOINCREMENT,
  "UnidadMedida" TEXT NOT NULL,
  "FechaCreacion" DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 2. CATEGORIAS (Abarrotes, Cremería, Botanas)
CREATE TABLE "Categorias" (
  "ID_Categoria" INTEGER PRIMARY KEY AUTOINCREMENT,
  "Categoria" TEXT NOT NULL,
  "Descripcion" TEXT NOT NULL,         -- obligatoria (para que cada categoría se explique sola)
  "Activo" INTEGER DEFAULT 1,
  "FechaCreacion" DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 3. PRODUCTOS
CREATE TABLE "Productos" (
  "ID_Producto" INTEGER PRIMARY KEY AUTOINCREMENT,
  "Producto" TEXT NOT NULL,
  "CodigoBarras" TEXT UNIQUE,
  "PrecioUnitario" REAL NOT NULL,
  "PrecioCosto" REAL NOT NULL,
  "Tipo" TEXT NOT NULL DEFAULT 'Producto', -- 'Producto' (controla inventario) | 'Servicio' (sin stock)
  "ManejaCaducidad" INTEGER DEFAULT 0,     -- 1 = el stock se lleva por lotes con caducidad (farmacia)
  "SeVendePeso" INTEGER DEFAULT 0,
  "StockMinimo" REAL DEFAULT 0,          -- NUEVO: alerta de "se está acabando"
  "ID_UnidadMedida" INTEGER,
  "ID_Categoria" INTEGER,
  "Activo" INTEGER DEFAULT 1,
  "FechaCreacion" DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY ("ID_UnidadMedida") REFERENCES "UnidadMedida" ("ID_UnidadMedida"),
  FOREIGN KEY ("ID_Categoria") REFERENCES "Categorias" ("ID_Categoria")
);

-- 4. INVENTARIO (Control de existencias - cantidad actual)
CREATE TABLE "Inventario" (
  "ID_Producto" INTEGER PRIMARY KEY,
  "Cantidad" REAL DEFAULT 0.0,
  "FechaModificacion" DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY ("ID_Producto") REFERENCES "Productos" ("ID_Producto")
);

-- =====================================================================
--  PERSONAS
-- =====================================================================

-- 5. USUARIOS (empleados que usan la caja)
CREATE TABLE "Usuarios" (
  "ID_Usuario" INTEGER PRIMARY KEY AUTOINCREMENT,
  "Nombre" TEXT NOT NULL,         -- Nombre completo del empleado
  "Usuario" TEXT NOT NULL UNIQUE, -- El login (ej: "juan99", "admin")
  "Contrasena" TEXT NOT NULL,     -- IMPORTANTE: guardar el HASH, no la contraseña en texto plano
  "Rol" TEXT NOT NULL,            -- 'Administrador' o 'Cajero'
  "Activo" INTEGER DEFAULT 1,     -- 1 = Puede entrar, 0 = Ya no trabaja ahí
  "FechaCreacion" DATETIME DEFAULT CURRENT_TIMESTAMP,
  "FechaModificacion" DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 6. CLIENTES (para el "fiado" / crédito a clientes frecuentes)
CREATE TABLE "Clientes" (
  "ID_Cliente" INTEGER PRIMARY KEY AUTOINCREMENT,
  "Nombre" TEXT NOT NULL,
  "Telefono" TEXT NOT NULL,         -- obligatorio
  "Email" TEXT,                     -- opcional
  "SaldoFiado" REAL DEFAULT 0.0,  -- cuánto debe a la fecha
  "Activo" INTEGER DEFAULT 1,
  "FechaCreacion" DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 7. PROVEEDORES (para el reabasto)
CREATE TABLE "Proveedores" (
  "ID_Proveedor" INTEGER PRIMARY KEY AUTOINCREMENT,
  "Proveedor" TEXT NOT NULL,
  "Contacto" TEXT,
  "Telefono" TEXT NOT NULL,         -- obligatorio
  "Email" TEXT,                     -- opcional
  "Activo" INTEGER DEFAULT 1,
  "FechaCreacion" DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- =====================================================================
--  CAJA (cortes y movimientos de efectivo)
-- =====================================================================

-- 8. METODOS DE PAGO
CREATE TABLE "MetodosPago" (
  "ID_MetodoPago" INTEGER PRIMARY KEY AUTOINCREMENT,
  "MetodoPago" TEXT NOT NULL UNIQUE,      -- 'Efectivo', 'Tarjeta Débito', 'Transferencia QR', 'Fiado'
  "Activo" INTEGER DEFAULT 1,             -- 1 = Visible en caja, 0 = Ya no se acepta
  "RequiereReferencia" INTEGER DEFAULT 0, -- 1 = Sí (folio de boucher/transferencia), 0 = No
  "FechaCreacion" DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 9. CORTES DE CAJA (apertura/cierre de turno)
CREATE TABLE "CortesCaja" (
  "ID_Corte" INTEGER PRIMARY KEY AUTOINCREMENT,
  "ID_Usuario" INTEGER NOT NULL,
  "FechaApertura" DATETIME DEFAULT CURRENT_TIMESTAMP,
  "FechaCierre" DATETIME,
  "MontoInicial" REAL NOT NULL,           -- fondo de caja con el que abre
  "MontoEsperado" REAL,                   -- lo que el sistema calcula que debe haber
  "MontoContado" REAL,                    -- lo que el cajero contó físicamente
  "Diferencia" REAL,                      -- sobrante/faltante
  "Estatus" TEXT DEFAULT 'Abierto',       -- 'Abierto' / 'Cerrado'
  FOREIGN KEY ("ID_Usuario") REFERENCES "Usuarios" ("ID_Usuario")
);

-- 10. MOVIMIENTOS DE CAJA (dinero que entra/sale sin ser venta)
CREATE TABLE "MovimientosCaja" (
  "ID_Movimiento" INTEGER PRIMARY KEY AUTOINCREMENT,
  "ID_Corte" INTEGER NOT NULL,
  "Tipo" TEXT NOT NULL,                   -- 'Ingreso' / 'Retiro'
  "Monto" REAL NOT NULL,
  "Concepto" TEXT,                        -- "Saqué para el gas", "Metí cambio", etc.
  "Fecha" DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY ("ID_Corte") REFERENCES "CortesCaja" ("ID_Corte")
);

-- =====================================================================
--  VENTAS
-- =====================================================================

-- 11. CABECERA DE VENTAS (El ticket global)
CREATE TABLE "Ventas" (
  "ID_Venta" INTEGER PRIMARY KEY AUTOINCREMENT,
  "Folio" TEXT,
  "ID_Usuario" INTEGER NOT NULL,
  "ID_MetodoPago" INTEGER NOT NULL,
  "ID_Corte" INTEGER,                     -- NUEVO: a qué turno pertenece la venta
  "ID_Cliente" INTEGER,                   -- NUEVO: solo si es venta a crédito (fiado)
  "ReferenciaPago" TEXT,                  -- folio de tarjeta/transferencia si aplica
  "FechaVenta" DATETIME DEFAULT CURRENT_TIMESTAMP,
  "Total" REAL NOT NULL,
  "PagoCon" REAL,
  "Cambio" REAL,
  "Estatus" TEXT DEFAULT 'Completada',    -- NUEVO: 'Completada' / 'Cancelada'
  FOREIGN KEY ("ID_Usuario") REFERENCES "Usuarios" ("ID_Usuario"),
  FOREIGN KEY ("ID_MetodoPago") REFERENCES "MetodosPago" ("ID_MetodoPago"),
  FOREIGN KEY ("ID_Corte") REFERENCES "CortesCaja" ("ID_Corte"),
  FOREIGN KEY ("ID_Cliente") REFERENCES "Clientes" ("ID_Cliente")
);

-- 12. DETALLE DE VENTAS (Los productos dentro de cada ticket)
CREATE TABLE "Detalle_Ventas" (
  "ID_Detalle" INTEGER PRIMARY KEY AUTOINCREMENT,
  "ID_Venta" INTEGER NOT NULL,
  "ID_Producto" INTEGER NOT NULL,
  "Cantidad" REAL NOT NULL,
  "PrecioVentaHistorico" REAL NOT NULL,   -- Respalda el precio del momento de la venta
  "CostoHistorico" REAL DEFAULT 0,        -- Costo de venta (COGS PEPS) de esta línea
  FOREIGN KEY ("ID_Venta") REFERENCES "Ventas" ("ID_Venta") ON DELETE CASCADE,
  FOREIGN KEY ("ID_Producto") REFERENCES "Productos" ("ID_Producto")
);

-- 13. ABONOS (pagos parciales del fiado)
CREATE TABLE "Abonos" (
  "ID_Abono" INTEGER PRIMARY KEY AUTOINCREMENT,
  "ID_Cliente" INTEGER NOT NULL,
  "ID_Venta" INTEGER,                     -- opcional: a qué venta fiada se aplica
  "ID_Usuario" INTEGER,                   -- quién recibió el pago
  "Monto" REAL NOT NULL,
  "Fecha" DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY ("ID_Cliente") REFERENCES "Clientes" ("ID_Cliente"),
  FOREIGN KEY ("ID_Venta") REFERENCES "Ventas" ("ID_Venta"),
  FOREIGN KEY ("ID_Usuario") REFERENCES "Usuarios" ("ID_Usuario")
);

-- =====================================================================
--  INVENTARIO: HISTORIAL Y COMPRAS
-- =====================================================================

-- 14. MOVIMIENTOS DE INVENTARIO (Kardex: el por qué cambió el stock)
CREATE TABLE "MovimientosInventario" (
  "ID_Movimiento" INTEGER PRIMARY KEY AUTOINCREMENT,
  "ID_Producto" INTEGER NOT NULL,
  "Tipo" TEXT NOT NULL,                   -- 'Entrada' / 'Venta' / 'Merma' / 'Ajuste'
  "Cantidad" REAL NOT NULL,               -- positivo = entra, negativo = sale
  "Motivo" TEXT,                          -- "Caducado", "Roto", "Ajuste conteo", etc.
  "ID_Usuario" INTEGER,
  "Fecha" DATETIME DEFAULT CURRENT_TIMESTAMP,
  "CostoUnitario" REAL DEFAULT 0,         -- Costo del movimiento, para el kardex valorizado
  -- El motivo es OBLIGATORIO en ajustes manuales y mermas (trazabilidad).
  -- Venta/Entrada (compra) traen su motivo fijo, así que no se ven afectadas.
  CHECK ("Tipo" NOT IN ('Ajuste','Merma') OR ("Motivo" IS NOT NULL AND TRIM("Motivo") <> '')),
  FOREIGN KEY ("ID_Producto") REFERENCES "Productos" ("ID_Producto"),
  FOREIGN KEY ("ID_Usuario") REFERENCES "Usuarios" ("ID_Usuario")
);

-- 14b. LOTES (caducidad/lote para farmacia; el stock vive por lote)
CREATE TABLE "Lotes" (
  "ID_Lote" INTEGER PRIMARY KEY AUTOINCREMENT,
  "ID_Producto" INTEGER NOT NULL,
  "Lote" TEXT,
  "Caducidad" TEXT,                       -- 'YYYY-MM-DD' (puede ser NULL)
  "Cantidad" REAL NOT NULL DEFAULT 0,
  "FechaCreacion" DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY ("ID_Producto") REFERENCES "Productos" ("ID_Producto")
);
CREATE INDEX IF NOT EXISTS idx_lotes_producto ON Lotes(ID_Producto);
CREATE INDEX IF NOT EXISTS idx_lotes_caducidad ON Lotes(Caducidad);

-- 14c. CAPAS DE COSTO (motor PEPS/FIFO: cada compra/alta crea una capa)
CREATE TABLE "CapasCosto" (
  "ID_Capa" INTEGER PRIMARY KEY AUTOINCREMENT,
  "ID_Producto" INTEGER NOT NULL,
  "Cantidad" REAL NOT NULL,               -- cantidad restante en la capa
  "CostoUnitario" REAL NOT NULL,
  "Fecha" DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY ("ID_Producto") REFERENCES "Productos" ("ID_Producto")
);
CREATE INDEX IF NOT EXISTS idx_capas_producto ON CapasCosto(ID_Producto);

-- 15. COMPRAS (cabecera del reabasto al proveedor)
CREATE TABLE "Compras" (
  "ID_Compra" INTEGER PRIMARY KEY AUTOINCREMENT,
  "ID_Proveedor" INTEGER,
  "ID_Usuario" INTEGER,
  "Folio" TEXT,                           -- folio/factura del proveedor
  "FechaCompra" DATETIME DEFAULT CURRENT_TIMESTAMP,
  "Total" REAL NOT NULL,
  FOREIGN KEY ("ID_Proveedor") REFERENCES "Proveedores" ("ID_Proveedor"),
  FOREIGN KEY ("ID_Usuario") REFERENCES "Usuarios" ("ID_Usuario")
);

-- 16. DETALLE DE COMPRAS (productos dentro de cada compra)
CREATE TABLE "Detalle_Compras" (
  "ID_Detalle" INTEGER PRIMARY KEY AUTOINCREMENT,
  "ID_Compra" INTEGER NOT NULL,
  "ID_Producto" INTEGER NOT NULL,
  "Cantidad" REAL NOT NULL,
  "CostoUnitario" REAL NOT NULL,          -- costo al que se compró (actualiza PrecioCosto)
  FOREIGN KEY ("ID_Compra") REFERENCES "Compras" ("ID_Compra") ON DELETE CASCADE,
  FOREIGN KEY ("ID_Producto") REFERENCES "Productos" ("ID_Producto")
);

-- =====================================================================
--  CONFIGURACIÓN
-- =====================================================================

-- 17. CONFIGURACION (datos del negocio para el ticket: llave-valor)
CREATE TABLE "Configuracion" (
  "Clave" TEXT PRIMARY KEY,               -- 'NombreTienda', 'Direccion', 'PieTicket', 'Telefono'
  "Valor" TEXT
);

-- 18. BITACORA (auditoría: quién hizo qué y cuándo)
CREATE TABLE "Bitacora" (
  "ID_Bitacora" INTEGER PRIMARY KEY AUTOINCREMENT,
  "ID_Usuario" INTEGER,
  "Usuario" TEXT,                          -- snapshot del login del usuario
  "Accion" TEXT NOT NULL,                  -- 'Alta'/'Edición'/'Venta'/'Cancelación'/'Cierre'…
  "Entidad" TEXT NOT NULL,                 -- 'Cliente'/'Producto'/'Venta'/'Corte'…
  "ID_Referencia" INTEGER,                 -- id del registro afectado
  "Detalle" TEXT,
  "Fecha" DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY ("ID_Usuario") REFERENCES "Usuarios" ("ID_Usuario")
);

-- 19. MONEDAS (catálogo de divisas: MXN, USD, EUR…)
CREATE TABLE "Monedas" (
  "ID_Moneda" INTEGER PRIMARY KEY AUTOINCREMENT,
  "Moneda" TEXT NOT NULL,                  -- 'Peso Mexicano'
  "Codigo" TEXT NOT NULL UNIQUE,           -- 'MXN'
  "Simbolo" TEXT,                          -- '$'
  "EsPrincipal" INTEGER DEFAULT 0,         -- 1 = moneda base de la tienda
  "Activo" INTEGER DEFAULT 1,
  "FechaCreacion" DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 20. DENOMINACIONES (billetes y monedas de cada divisa)
CREATE TABLE "Denominaciones" (
  "ID_Denominacion" INTEGER PRIMARY KEY AUTOINCREMENT,
  "ID_Moneda" INTEGER NOT NULL,
  "Valor" REAL NOT NULL,                   -- 500, 0.50…
  "Tipo" TEXT,                             -- 'Billete' / 'Moneda'
  "Activo" INTEGER DEFAULT 1,
  FOREIGN KEY ("ID_Moneda") REFERENCES "Monedas" ("ID_Moneda")
);

-- =====================================================================
--  LICENCIA (solo CACHÉ local para trabajar offline)
--  OJO: esta tabla NO valida nada por sí sola. La seguridad real vive en
--  el código Rust (verificación de firma) + tu servidor de licencias.
--  Nunca confíes en estos valores sin verificar TokenFirmado.
-- =====================================================================
CREATE TABLE "LicenciaLocal" (
  "ID" INTEGER PRIMARY KEY CHECK ("ID" = 1),  -- una sola fila siempre
  "Clave" TEXT,
  "TokenFirmado" TEXT,                    -- JWT/blob firmado por TU servidor
  "MachineID" TEXT,
  "FechaActivacion" DATETIME,
  "FechaExpiracion" DATETIME,
  "UltimoChequeoOnline" DATETIME,         -- para calcular el periodo de gracia
  "Estado" TEXT                           -- 'Activa'/'Expirada'/'SinActivar'
);

-- =====================================================================
--  ÍNDICES recomendados (búsquedas rápidas)
-- =====================================================================
CREATE INDEX "idx_productos_codigobarras" ON "Productos" ("CodigoBarras");
CREATE INDEX "idx_ventas_fecha"           ON "Ventas" ("FechaVenta");
CREATE INDEX "idx_ventas_corte"           ON "Ventas" ("ID_Corte");
CREATE INDEX "idx_detalleventas_venta"    ON "Detalle_Ventas" ("ID_Venta");
CREATE INDEX "idx_movinv_producto"        ON "MovimientosInventario" ("ID_Producto");
