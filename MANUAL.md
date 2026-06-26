# Free POS — Manual de uso

Punto de venta para tu negocio (abarrotes, farmacia, refaccionaria, papelería, peluquería, etc.). Este manual resume cómo operarlo día a día.

> También está disponible dentro de la app en **📖 Guía** (dashboard), con botón para **imprimir o guardar en PDF**.

---

## Roles

- **Administrador:** acceso total — productos, inventario, compras, proveedores, clientes, usuarios, catálogos, reportes y bitácora.
- **Cajero:** puede iniciar sesión, **abrir caja, vender** y registrar abonos/clientes; **no** modifica inventario ni catálogos.

## Primeros pasos

1. Crea el **usuario administrador** (configuración inicial, solo la primera vez).
2. Captura los **datos del negocio** (nombre, dirección, teléfono, pie de ticket) — aparecen en el ticket.
3. Elige un **catálogo de categorías** según tu giro (en el asistente inicial, o en Catálogos → Categorías → **📚 Catálogo por giro**).
4. Da de alta o **importa productos** (Inventario, o Catálogos → Importar/Exportar).

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

- Crea y **edita** productos: precio de venta, costo, código de barras, categoría, unidad, stock mínimo.
- **Ajustar existencias**: *Entrada* (reabasto), *Merma* (caducado/roto) o *Ajuste* (conteo físico real).
- Las existencias por debajo del mínimo se marcan en **rojo**.

## Clientes y fiado 🧾

- Registra clientes y consulta su **estado de cuenta** (compras a fiado y abonos).
- Registra **abonos** para bajar su deuda; el saldo se actualiza solo.

## Compras y proveedores 🚚

- Registra **compras** para reabastecer: suma al inventario, deja rastro en el kardex y opcionalmente **actualiza el precio de costo**.
- Administra tu **directorio de proveedores**.

## Reportes 📈

- Genera reportes de **ventas, inventario y cortes** por rango de fechas, en **Excel (XLSX), PDF o ambos**.

## Catálogos ⚙️

Administra: **categorías**, **unidades de medida**, **métodos de pago**, **monedas y denominaciones**, **datos del negocio** e **importar/exportar** (XLSX/CSV).

## Bitácora 📋

Registro de auditoría: **quién hizo qué y cuándo** (ventas, cancelaciones, altas, ediciones, cierres, importaciones…). Solo administradores.

## Importar / Exportar

- Acepta **.xlsx** y **.csv**, detectando la codificación (UTF-8 o Latin-1) para que los **acentos** se lean bien.
- Encabezados esperados (en MAYÚSCULAS):
  - **Productos:** `PRODUCTO`, `PRECIOVENTA` (obligatorias) + `CODIGOBARRAS`, `PRECIOCOSTO`, `EXISTENCIA`, `CATEGORIA`.
  - **Clientes:** `NOMBRE` (+ `TELEFONO`).
  - **Categorías:** `CATEGORIA` (+ `DESCRIPCION`).
  - **Proveedores:** `PROVEEDOR` (+ `CONTACTO`, `TELEFONO`).
- Plantillas de ejemplo en la carpeta `scripts/`.

## Licencia

- El sistema incluye una **prueba** al inicio (anclada al equipo). Después se activa con una **clave de producto**.

---

*Free POS — Aqua Studio.*
