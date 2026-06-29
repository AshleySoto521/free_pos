# AquaPOS

**Versión 1.0.0**

Punto de venta de escritorio para **comercios y negocios** (abarrotes, farmacia, refaccionaria, papelería, peluquería, etc.). App Windows (`.exe`) construida con **SvelteKit + Tauri**, base de datos **SQLite local** y **licenciamiento en Supabase**.

> Documento técnico para desarrolladores. El manual de usuario está en [`MANUAL.md`](./MANUAL.md) y dentro de la app en **📖 Guía**.

> **Versionado:** la versión se define en `src/lib/version.ts` (la que muestra el footer/Guía) y debe coincidir con `package.json`, `src-tauri/tauri.conf.json` (instalador/.exe) y `src-tauri/Cargo.toml`. SemVer `MAYOR.MENOR.PARCHE`.

---

## Stack

| Capa | Tecnología |
|---|---|
| Frontend | SvelteKit 2 · Svelte 5 (runes) · Tailwind CSS 4 · `adapter-static` (SPA) |
| Shell de escritorio | Tauri 2 (WebView2 en Windows) |
| Backend | Rust — comandos `#[tauri::command]` |
| Base de datos | SQLite (`rusqlite` + `r2d2`, bundled) en `%APPDATA%` |
| Licencias / prueba | Supabase (Postgres + Edge Functions Deno) · firma **Ed25519** |
| Contraseñas | Argon2 |
| Reportes / archivos | SheetJS (`xlsx`), jsPDF (+ autotable) |
| Gestor de paquetes | **pnpm** (obligatorio) |

---

## Arquitectura

```
┌─────────────────────────────────────────────┐
│  Ventana Tauri (WebView2)                     │
│  ┌───────────────────────────────────────┐    │
│  │ SvelteKit SPA (ssr=false)             │    │
│  │  rutas /vender, /inventario, …        │    │
│  │  src/lib/api.ts  ── invoke() ───┐     │    │
│  └─────────────────────────────────┼─────┘    │
│                                    ▼          │
│  Rust (commands.rs)  ── State<Db> (r2d2) ──► SQLite (pos.db)
│                      ── State<Sesion> (RBAC)  │
│                      ── reqwest ──────────────┼──► Supabase Edge Functions
└─────────────────────────────────────────────┘         (activar / trial)
```

- **Toda la lógica de datos vive en Rust** (queries parametrizadas). El front solo llama `invoke()` vía el wrapper tipado `src/lib/api.ts`.
- **RBAC real**: la sesión y el rol se guardan en el backend (`State<Sesion>`), no en el front. Los comandos sensibles llaman a `exigir_admin()`.
- **La seguridad de licencias vive en Rust + servidor**: el `.exe` solo trae la llave **pública** Ed25519 y verifica tokens firmados; la privada vive en Supabase.

---

## Estructura

```
aquapos/
├── src/                          # Frontend SvelteKit
│   ├── routes/                   # una carpeta por pantalla (+page.svelte)
│   │   ├── +layout.svelte        # monta <Gate> (portero) + <Toast>
│   │   ├── +page.svelte          # dashboard
│   │   ├── vender/ corte/ inventario/ ventas/ compras/ proveedores/
│   │   ├── clientes/ usuarios/ bitacora/ reportes/ licencia/
│   │   ├── catalogos/ categorias/ unidades/ metodos-pago/ monedas/
│   │   ├── configuracion/ datos/ bienvenida/ ayuda/
│   └── lib/
│       ├── api.ts                # wrapper tipado de TODOS los comandos
│       ├── format.ts             # pesos() + fechas en hora local
│       ├── xlsx.ts  pdf.ts  ticket.ts   # exportar/importar, PDF, ticket 80mm
│       ├── components/           # Gate, Login, SetupAdmin, Activar,
│       │                         #   Toast, ContadorMonedas
│       └── stores/               # session.ts, toast.ts
├── src-tauri/                    # Backend Rust
│   ├── src/
│   │   ├── main.rs  lib.rs       # entry + Builder (manage Db/Sesion, handlers)
│   │   ├── db.rs                 # pool, esquema embebido, migraciones, siembra
│   │   ├── models.rs             # structs serde (camelCase) FE↔BE
│   │   ├── commands.rs           # todos los #[tauri::command]
│   │   └── license.rs            # Ed25519, machine_id, trial, llamadas de red
│   ├── Cargo.toml  tauri.conf.json  capabilities/
├── supabase/functions/           # Edge Functions (Deno)
│   ├── activar/   trial/         # firman tokens (licencia / prueba)
│   └── deno.json
├── scripts/                      # SQL + plantillas CSV
│   ├── posdb.sql                 # esquema local (referencia; se embebe en db.rs)
│   ├── licencias_supabase.sql    generar_licencias.sql  trial_supabase.sql
│   ├── reset_supabase.sql        # drop de objetos para recrear
│   └── productos.csv clientes.csv categorias.csv proveedores.csv
├── tools/generar_llaves.mjs      # genera par Ed25519
├── MANUAL.md                     # manual de usuario
└── pnpm-workspace.yaml           # config pnpm (allowBuilds: core-js)
```

---

## Funcionalidad

- **Vender** — escaneo de código de barras, carrito, cliente o "Público en General", métodos de pago (efectivo con cambio, tarjeta/transferencia con referencia, fiado), **divisa configurable** ("$100.00 MXN"), reloj en vivo, **ticket térmico 80 mm** (imprimir o PDF). Requiere caja abierta.
- **Multi-giro** — vende **productos** (con inventario), **servicios** (sin inventario, ej. mano de obra) o **ambos**; el modo se elige en Configuración. Productos con **caducidad/lote** (farmacia).
- **Corte de caja** — abrir/cerrar turno con **contador obligatorio de denominaciones**, ingresos/retiros, cuadre esperado vs contado.
- **Inventario (costeo PEPS/FIFO)** — alta/edición; el **stock entra solo por Compras** (o carga inicial), no en el alta. Ajustes manuales: **Conteo físico** y **Merma** con **motivo obligatorio**; el stock encontrado toma el costo del último lote (o el que se indique) y la merma descuenta del lote más viejo. **Kardex valorizado** por producto (costo + saldo, exportable). Alerta de stock bajo. Lotes con caducidad y avisos por vencer.
- **Ventas** — historial + cierre por rango, detalle, **cancelar venta** (revierte inventario —regresar/merma/descartar— y fiado).
- **Compras / Proveedores** — reabasto que crea **capas de costo PEPS** y suma inventario; **candado anti-duplicados** (misma factura no se carga 2 veces, incluso sin folio); alta de varios productos a la vez e **importación de factura** (XLSX/CSV) con `PRECIOVENTA` opcional. `PrecioCosto` se actualiza solo como "último costo de referencia". Directorio de proveedores CRUD (teléfono obligatorio + email).
- **Clientes / Fiado** — CRUD con **teléfono obligatorio y email**, estado de cuenta (cargos/abonos), abonos.
- **Usuarios** — CRUD, roles, cambio de contraseña (Argon2), salvaguarda de "último admin".
- **Catálogos** — categorías, unidades, métodos de pago, **monedas y denominaciones**, datos del negocio, importar/exportar con **plantillas .csv descargables**.
- **Reportes** — ventas/inventario/cortes + **utilidad (PEPS)** e **inventario valorizado**, en **XLSX, PDF o ambos**.
- **Bitácora** — auditoría de acciones por usuario.
- **Onboarding** — wizard (datos del negocio → divisa → modo de venta → catálogo por giro) + Guía imprimible.
- **Licencia** — prueba anclada al equipo + activación por clave.

### RBAC

- **Administrador:** todo.
- **Cajero:** login, abrir caja, vender, crear clientes y editar su info personal, abonos. No toca inventario/catálogos/usuarios/cancelaciones.

Enforced en backend (`exigir_admin`), en el dashboard (tarjetas filtradas) y con guardas de ruta.

---

## Modelo de datos (SQLite local)

`UnidadMedida`, `Categorias`, `Productos`, `Inventario`, `Usuarios`, `Clientes`,
`Proveedores`, `MetodosPago`, `CortesCaja`, `MovimientosCaja`, `Ventas`,
`Detalle_Ventas`, `Abonos`, `MovimientosInventario` (kardex), `Lotes` (caducidad),
`CapasCosto` (motor PEPS/FIFO), `Compras`, `Detalle_Compras`,
`Configuracion` (llave-valor), `Bitacora`, `Monedas`, `Denominaciones`,
`LicenciaLocal` (caché del token firmado).

- El esquema está en `scripts/posdb.sql` y **se embebe** en el binario vía `include_str!` (`db.rs`); es la **fuente de verdad** de las bases nuevas.
- Columnas clave: `Productos.Tipo` (Producto/Servicio), `ManejaCaducidad`, `SeVendePeso`; `Detalle_Ventas.CostoHistorico` (COGS PEPS); `MovimientosInventario.CostoUnitario`; `Clientes/Proveedores.Email`.
- **PEPS/FIFO**: cada compra/carga crea una capa en `CapasCosto`; la venta consume la más vieja y guarda el costo en `Detalle_Ventas.CostoHistorico`. La existencia operativa vive en `Inventario`/`Lotes` (separada del costeo, para que un error de costo no afecte el stock).
- **Trazabilidad**: `MovimientosInventario` tiene `CHECK` que **exige `Motivo`** en `Ajuste`/`Merma`.
- Para bases ya existentes, `db.rs` aplica migraciones idempotentes (`CREATE TABLE IF NOT EXISTS`, `agregar_columna_si_falta`).
- Siembra inicial: unidades, métodos de pago, Peso MXN + denominaciones (las categorías se eligen por giro en el onboarding).
- **Fechas**: se guardan en UTC (`CURRENT_TIMESTAMP`); se filtran con `date(col,'localtime')` y se muestran en hora local (`format.ts`).

---

## Requisitos de desarrollo

1. **Rust** (rustup, toolchain MSVC) + **Microsoft C++ Build Tools** (Visual Studio Build Tools, workload "Desktop development with C++").
2. **Node.js** + **pnpm** (`packageManager` fijado en `package.json`).
3. **WebView2** (incluido en Windows 11).

> Usar **siempre pnpm**, nunca npm (mezclarlos rompe el árbol de `node_modules`).
> Si pnpm pide aprobar build scripts (`core-js`), está resuelto en `pnpm-workspace.yaml` (`allowBuilds`); o corre `pnpm approve-builds`.

## Configuración de secretos (build)

Los datos del despliegue (URL de Supabase, anon key, llave pública de licencias) **no** viven en el código: se inyectan al compilar desde variables de entorno definidas en `src-tauri/.cargo/config.toml`, archivo **gitignoreado**. Tras clonar:

```sh
cp src-tauri/.cargo/config.toml.example src-tauri/.cargo/config.toml
# edita config.toml con tus valores reales
```

- `AQUAPOS_SUPABASE_URL` — Supabase → Project Settings → API → Project URL.
- `AQUAPOS_SUPABASE_ANON_KEY` — Supabase → Project Settings → API → anon public.
- `AQUAPOS_LICENSE_PUBLIC_KEY` — llave pública (hex) del par Ed25519. La **privada** vive solo en Supabase Secrets (`LICENSE_PRIVATE_KEY`), nunca en el repo ni en el `.exe`.

Sin configurar, la app compila y corre en **modo local** (trial sin servidor de licencias).

## Comandos

```sh
pnpm install            # dependencias JS
pnpm tauri dev          # app en desarrollo (compila Rust + levanta Vite)
pnpm tauri build        # genera el instalador .exe (requiere iconos)
pnpm tauri icon logo.png   # genera iconos desde un PNG cuadrado

pnpm check              # svelte-check (tipos)
pnpm lint               # prettier + eslint
pnpm build              # build del front (SPA estática -> /build)
```

`tauri.conf.json` → `frontendDist: ../build`, `devUrl: http://localhost:5173`.
`vite.config.ts` ignora `src-tauri/` en el watcher y fija el puerto 5173.

---

## Base de datos local: ubicación y reset

```
%APPDATA%\com.aquastudio.aquapos\pos.db        (+ -wal, -shm)
```

Reset (con la app cerrada):

```powershell
Remove-Item "$env:APPDATA\com.aquastudio.aquapos\pos.db*" -Force
```

Al reiniciar, `db.rs` recrea esquema + migraciones + siembra.

---

## Licenciamiento (Supabase)

**Diseño:** el servidor firma tokens (licencia o prueba) con Ed25519; el `.exe`
verifica con la llave pública embebida. La **prueba se ancla por `machine_id`**
(hash de hardware) para que borrar la DB local no la reinicie.

**Puesta en marcha:**

1. SQL Editor → correr `licencias_supabase.sql`, `generar_licencias.sql`, `trial_supabase.sql`
   (para recrear desde cero, primero `reset_supabase.sql`).
2. `node tools/generar_llaves.mjs` → pega la **pública** en `license.rs` (`SERVER_PUBLIC_KEY_HEX`); guarda la **privada** como secret `LICENSE_PRIVATE_KEY`.
3. Desplegar funciones:
   ```sh
   supabase functions deploy activar --no-verify-jwt
   supabase functions deploy trial   --no-verify-jwt
   ```
4. Generar claves de venta: `select fn_generar_licencia('Anual', 'correo', 'Nombre');`

> Mientras `SERVER_PUBLIC_KEY_HEX` siga en ceros (placeholder), la **prueba funciona en modo local** (desarrollo). Con la llave real puesta, se activa la prueba anclada al servidor.

**Edge Functions (Deno):** el editor usa la extensión Deno solo en `supabase/functions` (ver `.vscode/settings.json`); el resto del repo es Node/TS.

---

## Importar / Exportar catálogos

- Acepta `.xlsx` y `.csv`; detecta codificación (**UTF-8 o Latin-1**) para leer acentos correctamente (`xlsx.ts`).
- **Plantillas .csv descargables** desde cada sección (botón "⬇️ Descargar plantilla"), con encabezados exactos + fila de ejemplo (BOM UTF-8 para Excel). Helper `descargarPlantillaCSV()`.
- Validación en dos niveles: si **falta una columna obligatoria** o un **renglón viene incompleto**, se rechaza el archivo entero y se listan los errores (corregir en Excel y resubir). Todo es admin-only, parametrizado y queda en bitácora.
- **Tres herramientas de productos** (Catálogos → Importar/Exportar), sin enciman­se:
  - **Agregar productos (catálogo)** — alta masiva de productos nuevos con **0 existencia** (no toca stock ni PEPS). Cols: `PRODUCTO`, `PRECIOVENTA`, `PRECIOCOSTO`, `CATEGORIA`, `UNIDAD` (+ `CODIGOBARRAS`, `SEVENDEPESO`).
  - **Iniciar inventario** — carga inicial al inaugurar: **resetea y siembra** existencia + capas PEPS. Añade `EXISTENCIA` a las columnas anteriores.
  - **Actualizar precios** — cambia precios en masa sin tocar stock. Cols: `PRODUCTO`, `PRECIOVENTA` (+ `CODIGOBARRAS`, `PRECIOCOSTO`).
- Otros importables: **clientes** (`NOMBRE`, `TELEFONO` obligatorios + `EMAIL`), **proveedores** (`PROVEEDOR`, `TELEFONO` obligatorios + `CONTACTO`, `EMAIL`), **categorías** (`CATEGORIA` + `DESCRIPCION`).
- **Compras** importa la factura: `PRODUCTO`/`CODIGOBARRAS` + `PRECIOCOMPRA` + `CANTIDAD` (+ `PRECIOVENTA`, `LOTE`, `CADUCIDAD`). Si un producto no existe, **bloquea** el registro (no se crean productos desde compras).

---

## Convenciones

- **Modelos**: serde `rename_all = "camelCase"` → el front recibe/manda camelCase; Tauri convierte las llaves de argumentos a snake_case.
- **Dinero**: `f64` (REAL). Para una tiendita es suficiente; si se requiere exactitud contable, migrar a enteros (centavos).
- **Errores de comando**: `Result<T, String>`.
- **Sin hard-deletes** en catálogos principales (se usa `Activo`); las denominaciones sí se borran.

---

## Contacto

Soporte técnico, compras o dudas: **contactoaquastudio@gmail.com**

© Aqua Studio 2026 — Todos los derechos reservados · AquaPOS v1.0.0
