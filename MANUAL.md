# AquaPOS — Manual de uso

**Versión 1.0.0**

Punto de venta para tu negocio (abarrotes, farmacia, refaccionaria, papelería, peluquería, etc.). Este manual resume cómo operarlo día a día.

> También está disponible dentro de la app en **📖 Guía** (dashboard), con botón para **imprimir o guardar en PDF**. La versión aparece en el pie de página de la app.

---

## Roles

- **Administrador:** acceso total — productos, inventario, compras, proveedores, clientes, usuarios, catálogos, reportes y bitácora.
- **Cajero:** puede iniciar sesión, **abrir caja, vender** y registrar abonos/clientes; **no** modifica inventario ni catálogos.

## Primeros pasos

1. Crea el **usuario administrador** (configuración inicial, solo la primera vez).
2. Captura los **datos del negocio** (nombre, dirección, teléfono, pie de ticket) — aparecen en el ticket.
3. Elige tu **moneda** y tu **modo de venta** (productos, servicios o ambos) en el asistente inicial o en **Configuración**.
4. Elige un **catálogo de categorías** según tu giro (asistente inicial, o Catálogos → Categorías → **📚 Catálogo por giro**).
5. Carga tu inventario: en **Catálogos → Importar/Exportar → 🏁 Iniciar inventario** sube tu existencia inicial de una vez (solo al inaugurar). Después, el stock entra por **Compras**.

## Vender 🛒

- **Primero abre la caja**: cuenta billetes y monedas; ese total es tu fondo inicial. No se puede vender sin caja abierta.
- Escanea el **código de barras** (o búscalo por nombre) para agregar productos al ticket.
- Elige el **cliente** (o "Público en General") y el **método de pago**. En efectivo se calcula el **cambio**.
- Para **fiado**, selecciona un cliente registrado (se le carga a su saldo).
- Al cobrar, **imprime el ticket** (impresora térmica 80 mm) o **guárdalo en PDF**.

## Corte de caja 💵

- Al **abrir** y **cerrar** turno se cuenta el efectivo **por denominación** (billetes y monedas).
- El **cierre** compara lo *esperado* (fondo + ventas en efectivo + ingresos − retiros) contra lo *contado* y muestra la **diferencia** (sobra/falta/cuadra).
- Usa **Ingreso / Retiro** para movimientos de efectivo que no son ventas (ej. "saqué para el gas").

## Inventario 📦

- Crea y **edita** productos: precio de venta, código de barras, categoría, unidad, stock mínimo, si **se vende por peso** y si **maneja caducidad**.
- **El producto se crea con 0 existencia.** El stock entra por **Compras** (o por la carga inicial). Así el costo y las existencias tienen un solo origen y no se duplican.
- **Los 3 caminos del stock:**
  - **Iniciar inventario** (Importar/Exportar) → solo al inaugurar, carga lo que ya tienes.
  - **Compras** → el día a día, reabastecer.
  - **Ajustar** → correcciones manuales.
- **Ajustar existencias** (botón Ajustar): *Conteo físico* (pones la existencia real) o *Merma/Baja* (caducado, roto, robo…). El **motivo es obligatorio** para saber por qué cambió el stock. Si encontraste mercancía, puedes indicar su costo (o usa el del último lote); la merma descuenta del lote más viejo.
- **Costeo PEPS**: el sistema lleva el costo por capas (primero en entrar, primero en salir). Puedes ver el **kardex valorizado** de cada producto (botón Historial) con costo y saldo, y exportarlo.
- **Caducidad/lote** (farmacia): el stock se lleva por lotes con fecha; hay avisos de **próximos a vencer**.
- Las existencias por debajo del mínimo se marcan en **rojo**.

## Clientes y fiado 🧾

- Registra clientes con **nombre y teléfono obligatorios** y **correo electrónico** opcional.
- Consulta su **estado de cuenta** (compras a fiado y abonos) y registra **abonos** para bajar su deuda; el saldo se actualiza solo.

## Compras y proveedores 🚚

- Registra **compras** para reabastecer: suma al inventario y crea la **capa de costo PEPS** de esa compra. El precio de costo de referencia se actualiza solo (ya no se pregunta).
- Puedes **agregar varios productos a la vez** o **importar la factura** desde Excel/CSV. Si la factura trae **precio de venta**, también lo actualiza.
- **Anti-duplicados**: si intentas cargar la misma factura dos veces (incluso sin folio), el sistema lo detecta y lo evita. Solo se registran facturas **completas**: si un producto no existe aún, créalo primero.
- Administra tu **directorio de proveedores** (nombre y teléfono obligatorios + correo opcional).

## Reportes 📈

- Genera reportes de **ventas, inventario y cortes** por rango de fechas, en **Excel (XLSX), PDF o ambos**.
- Incluye **utilidad** (con el costo PEPS) e **inventario valorizado**.

## Catálogos ⚙️

Administra: **categorías**, **unidades de medida**, **métodos de pago**, **monedas y denominaciones**, **datos del negocio** e **importar/exportar** (XLSX/CSV).

## Bitácora 📋

Registro de auditoría: **quién hizo qué y cuándo** (ventas, cancelaciones, altas, ediciones, cierres, importaciones…). Solo administradores.

## Importar / Exportar

- Acepta **.xlsx** y **.csv**, detectando la codificación (UTF-8 o Latin-1) para que los **acentos** se lean bien.
- Cada sección tiene un botón **⬇️ Descargar plantilla .csv** con las columnas exactas y un ejemplo: bájala, llénala en Excel y súbela.
- Si falta una columna o un renglón está incompleto, **no se importa nada** y te dice exactamente qué corregir (arréglalo en Excel y vuelve a subir).
- **Tres formas de cargar productos** (no se enciman):
  - **Agregar productos (catálogo)** — para **expansión**: da de alta productos nuevos con 0 existencia (el stock entra luego por Compras). Columnas: `PRODUCTO`, `PRECIOVENTA`, `PRECIOCOSTO`, `CATEGORIA`, `UNIDAD` + opcionales `CODIGOBARRAS`, `SEVENDEPESO` (Sí/No).
  - **🏁 Iniciar inventario** — **solo al inaugurar**: carga tu existencia inicial (resetea y siembra). Agrega `EXISTENCIA`.
  - **💲 Actualizar precios** — cambia precios en masa sin tocar existencias. Columnas: `PRODUCTO`, `PRECIOVENTA` (+ `PRECIOCOSTO`).
- Otros archivos:
  - **Clientes:** `NOMBRE` y `TELEFONO` (obligatorios) + `EMAIL`.
  - **Proveedores:** `PROVEEDOR` y `TELEFONO` (obligatorios) + `CONTACTO`, `EMAIL`.
  - **Categorías:** `CATEGORIA` (+ `DESCRIPCION`).

## Licencia

- El sistema incluye una **prueba** al inicio (anclada al equipo). Después se activa con una **clave de producto**.

---

## Contacto

Soporte técnico, compras o dudas: **contactoaquastudio@gmail.com**

© Aqua Studio 2026 — Todos los derechos reservados - AquaPOS v1.0.0
