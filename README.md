# AquaPOS

Punto de venta de escritorio para **comercios y negocios** (abarrotes, farmacia, refaccionaria, papelería, peluquería, etc.). App Windows (`.exe`) construida con **SvelteKit + Tauri**, base de datos **SQLite local** y **licenciamiento en Supabase**.

> Documento técnico para desarrolladores. El manual de usuario está en [`MANUAL.md`](./MANUAL.md) y dentro de la app en **📖 Guía**.

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

- **Vender** — escaneo de código de barras, carrito, cliente o "Público en General", métodos de pago (efectivo con cambio, tarjeta/transferencia con referencia, fiado), **ticket térmico 80 mm** (imprimir o PDF). Requiere caja abierta.
- **Corte de caja** — abrir/cerrar turno con **contador obligatorio de denominaciones**, ingresos/retiros, cuadre esperado vs contado.
- **Inventario** — alta/edición de productos, activar/desactivar, ajuste de existencias (Entrada/Merma/Ajuste), alerta de stock bajo.
- **Ventas** — historial + cierre por rango, detalle, **cancelar venta** (revierte inventario y fiado).
- **Compras / Proveedores** — reabasto (suma inventario + actualiza costo), directorio CRUD.
- **Clientes / Fiado** — CRUD, estado de cuenta (cargos/abonos), abonos.
- **Usuarios** — CRUD, roles, cambio de contraseña (Argon2), salvaguarda de "último admin".
- **Catálogos** — categorías, unidades, métodos de pago, **monedas y denominaciones**, datos del negocio, importar/exportar.
- **Reportes** — ventas/inventario/cortes en **XLSX, PDF o ambos**.
- **Bitácora** — auditoría de acciones por usuario.
- **Onboarding** — wizard (datos del negocio → catálogo por giro) + Guía imprimible.
- **Licencia** — prueba anclada al equipo + activación por clave.

### RBAC

- **Administrador:** todo.
- **Cajero:** login, abrir caja, vender, crear clientes y editar su info personal, abonos. No toca inventario/catálogos/usuarios/cancelaciones.

Enforced en backend (`exigir_admin`), en el dashboard (tarjetas filtradas) y con guardas de ruta.

---

## Modelo de datos (SQLite local)

`UnidadMedida`, `Categorias`, `Productos`, `Inventario`, `Usuarios`, `Clientes`,
`Proveedores`, `MetodosPago`, `CortesCaja`, `MovimientosCaja`, `Ventas`,
`Detalle_Ventas`, `Abonos`, `MovimientosInventario` (kardex), `Compras`,
`Detalle_Compras`, `Configuracion` (llave-valor), `Bitacora`, `Monedas`,
`Denominaciones`, `LicenciaLocal` (caché del token firmado).

- El esquema está en `scripts/posdb.sql` y **se embebe** en el binario vía `include_str!` (`db.rs`).
- Tablas nuevas (`Bitacora`, `Monedas`, `Denominaciones`) se crean con `CREATE TABLE IF NOT EXISTS` en `db.rs` (migración para bases ya existentes).
- Siembra inicial: unidades, métodos de pago, categorías comunes, Peso MXN + denominaciones.
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
- Importables: productos, clientes, categorías, proveedores. Validan encabezados (MAYÚSCULAS), normalizan (trim + UPPERCASE), insertan parametrizado, son admin-only y quedan en bitácora.
- Plantillas de ejemplo en `scripts/*.csv`.

---

## Convenciones

- **Modelos**: serde `rename_all = "camelCase"` → el front recibe/manda camelCase; Tauri convierte las llaves de argumentos a snake_case.
- **Dinero**: `f64` (REAL). Para una tiendita es suficiente; si se requiere exactitud contable, migrar a enteros (centavos).
- **Errores de comando**: `Result<T, String>`.
- **Sin hard-deletes** en catálogos principales (se usa `Activo`); las denominaciones sí se borran.

---

## Contacto

Soporte técnico, compras o dudas: **contactoaquastudio@gmail.com**

© Aqua Studio 2026 — Todos los derechos reservados
