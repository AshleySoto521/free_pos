-- =====================================================================
--  SERVIDOR DE LICENCIAS para SUPABASE (Postgres)
--  Pega esto en:  Supabase -> SQL Editor -> New query -> Run
--
--  Arquitectura:
--   - Estas tablas son la FUENTE DE VERDAD (no viven en el .exe).
--   - RLS bloquea todo para anon/authenticated. Solo el service_role
--     (que usa tu Edge Function) puede leerlas/escribirlas.
--   - La función fn_activar_licencia() hace la activación de forma ATÓMICA.
--   - La Edge Function llama a esa función, recibe el payload y lo FIRMA
--     con Ed25519 (la llave privada vive en los Secrets de Supabase).
-- =====================================================================

create extension if not exists pgcrypto;

-- ---------------------------------------------------------------------
-- 1. COMPRADORES (quienes te compran el POS)
-- ---------------------------------------------------------------------
create table lic_compradores (
  id             bigint generated always as identity primary key,
  nombre         text not null,
  email          text unique,
  telefono       text,
  notas          text,
  fecha_registro timestamptz not null default now()
);

-- ---------------------------------------------------------------------
-- 2. PLANES / TIPOS DE LICENCIA
-- ---------------------------------------------------------------------
create table lic_planes (
  id               bigint generated always as identity primary key,
  plan             text not null,                 -- 'Mensual','Anual','Perpetua'
  precio           numeric(10,2),
  max_activaciones int  not null default 1,       -- en cuántas PCs a la vez
  dias_vigencia    int,                            -- null = perpetua
  dias_gracia      int  not null default 7,        -- días que opera offline
  activo           boolean not null default true
);

-- ---------------------------------------------------------------------
-- 3. LICENCIAS (las claves emitidas)
-- ---------------------------------------------------------------------
create table licencias (
  id               bigint generated always as identity primary key,
  clave            text not null unique,           -- 'POS-XXXX-XXXX-XXXX-XXXX'
  comprador_id     bigint references lic_compradores(id),
  plan_id          bigint references lic_planes(id),
  max_activaciones int,                             -- override de dispositivos (null = usar el del plan)
  estado           text not null default 'Emitida',-- Emitida/Activa/Suspendida/Cancelada
  fecha_emision    timestamptz not null default now(),
  fecha_expiracion timestamptz,                     -- null = perpetua o aún sin activar
  notas            text
);

-- ---------------------------------------------------------------------
-- 4. ACTIVACIONES (qué máquina usa qué licencia)
-- ---------------------------------------------------------------------
create table lic_activaciones (
  id                bigint generated always as identity primary key,
  licencia_id       bigint not null references licencias(id) on delete cascade,
  machine_id        text not null,                  -- huella de la PC
  nombre_equipo     text,
  sistema_operativo text,
  version_app       text,
  fecha_activacion  timestamptz not null default now(),
  ultimo_chequeo    timestamptz,
  estado            text not null default 'Activa', -- Activa/Revocada
  unique (licencia_id, machine_id)
);

-- ---------------------------------------------------------------------
-- 5. BITÁCORA (auditoría / antifraude)
-- ---------------------------------------------------------------------
create table lic_log_validaciones (
  id          bigint generated always as identity primary key,
  licencia_id bigint references licencias(id),
  machine_id  text,
  accion      text,                                 -- Activar/Revalidar
  resultado   text,                                 -- OK/LimiteActivaciones/Expirada/...
  ip          text,
  fecha       timestamptz not null default now()
);

create index idx_licencias_clave       on licencias (clave);
create index idx_activaciones_licencia on lic_activaciones (licencia_id);
create index idx_activaciones_machine  on lic_activaciones (machine_id);

-- =====================================================================
--  ROW LEVEL SECURITY
--  Sin policies = todo bloqueado para anon/authenticated.
--  service_role (Edge Function) OMITE RLS, así que sí puede operar.
-- =====================================================================
alter table lic_compradores      enable row level security;
alter table lic_planes           enable row level security;
alter table licencias            enable row level security;
alter table lic_activaciones     enable row level security;
alter table lic_log_validaciones enable row level security;

-- =====================================================================
--  FUNCIÓN DE ACTIVACIÓN (atómica)
--  La Edge Function la invoca por RPC con el service_role. Hace toda la
--  validación bajo un FOR UPDATE para que el conteo de cupo sea seguro
--  ante activaciones simultáneas. Devuelve el payload SIN firmar; la
--  firma Ed25519 la pone la Edge Function.
--
--  Los nombres del payload coinciden con LicensePayload en license.rs:
--    clave, machine_id, plan, fecha_activacion, fecha_expiracion, dias_gracia
-- =====================================================================
create or replace function fn_activar_licencia(
  p_clave             text,
  p_machine_id        text,
  p_nombre_equipo     text default null,
  p_sistema_operativo text default null,
  p_version_app       text default null
)
returns jsonb
language plpgsql
security definer
set search_path = public
as $$
declare
  v_lic      licencias%rowtype;
  v_plan     lic_planes%rowtype;
  v_existing lic_activaciones%rowtype;
  v_count    int;
  v_max      int;
  v_exp      timestamptz;
begin
  -- 1) Buscar y BLOQUEAR la licencia (evita carreras en el conteo)
  select * into v_lic from licencias where clave = p_clave for update;
  if not found then
    insert into lic_log_validaciones(machine_id, accion, resultado)
      values (p_machine_id, 'Activar', 'ClaveInvalida');
    return jsonb_build_object('ok', false, 'error', 'ClaveInvalida');
  end if;

  if v_lic.estado in ('Suspendida','Cancelada') then
    insert into lic_log_validaciones(licencia_id, machine_id, accion, resultado)
      values (v_lic.id, p_machine_id, 'Activar', v_lic.estado);
    return jsonb_build_object('ok', false, 'error', v_lic.estado);
  end if;

  select * into v_plan from lic_planes where id = v_lic.plan_id;

  -- Dispositivos permitidos: el override de la licencia gana sobre el del plan
  v_max := coalesce(v_lic.max_activaciones, v_plan.max_activaciones, 1);

  -- 2) Calcular expiración en la PRIMERA activación si el plan es por tiempo
  v_exp := v_lic.fecha_expiracion;
  if v_exp is null and v_plan.dias_vigencia is not null then
    v_exp := now() + make_interval(days => v_plan.dias_vigencia);
  end if;

  -- 3) ¿Ya expiró?
  if v_exp is not null and now() > v_exp then
    insert into lic_log_validaciones(licencia_id, machine_id, accion, resultado)
      values (v_lic.id, p_machine_id, 'Activar', 'Expirada');
    return jsonb_build_object('ok', false, 'error', 'Expirada');
  end if;

  -- 4) ¿Esta máquina ya estaba activada? -> solo refrescamos el chequeo
  select * into v_existing from lic_activaciones
    where licencia_id = v_lic.id and machine_id = p_machine_id;

  if found then
    update lic_activaciones
      set ultimo_chequeo = now(),
          estado         = 'Activa',
          version_app    = coalesce(p_version_app, version_app)
      where id = v_existing.id;
  else
    -- 5) Máquina nueva -> revisar cupo
    select count(*) into v_count from lic_activaciones
      where licencia_id = v_lic.id and estado = 'Activa';
    if v_count >= v_max then
      insert into lic_log_validaciones(licencia_id, machine_id, accion, resultado)
        values (v_lic.id, p_machine_id, 'Activar', 'LimiteActivaciones');
      return jsonb_build_object('ok', false, 'error', 'LimiteActivaciones');
    end if;
    insert into lic_activaciones(
        licencia_id, machine_id, nombre_equipo, sistema_operativo, version_app, ultimo_chequeo)
      values (v_lic.id, p_machine_id, p_nombre_equipo, p_sistema_operativo, p_version_app, now());
  end if;

  -- 6) Marcar licencia activa y fijar su expiración (si se calculó)
  update licencias set estado = 'Activa', fecha_expiracion = v_exp where id = v_lic.id;

  insert into lic_log_validaciones(licencia_id, machine_id, accion, resultado)
    values (v_lic.id, p_machine_id, 'Activar', 'OK');

  -- 7) Payload que la Edge Function firmará con Ed25519
  return jsonb_build_object(
    'ok', true,
    'payload', jsonb_build_object(
      'clave',            v_lic.clave,
      'machine_id',       p_machine_id,
      'plan',             coalesce(v_plan.plan, 'Desconocido'),
      'fecha_activacion', now(),
      'fecha_expiracion', v_exp,
      'dias_gracia',      coalesce(v_plan.dias_gracia, 7)
    )
  );
end;
$$;

-- Que NADIE pueda llamarla desde la API pública (anon/authenticated).
-- Solo el service_role de la Edge Function.
revoke execute on function fn_activar_licencia(text,text,text,text,text) from public, anon, authenticated;

-- =====================================================================
--  DATOS DE EJEMPLO
-- =====================================================================
insert into lic_planes (plan, precio, max_activaciones, dias_vigencia, dias_gracia) values
  ('Mensual',   199.00, 1, 30,   7),
  ('Anual',    1999.00, 1, 365, 14),
  ('Perpetua', 4999.00, 1, null, 30);
