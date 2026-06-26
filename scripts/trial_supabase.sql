-- =====================================================================
--  PRUEBA (TRIAL) ANCLADA AL SERVIDOR — anti-reset
--  Correr en Supabase -> SQL Editor (después de licencias_supabase.sql).
--
--  Idea: la fecha de inicio de la prueba se guarda en el servidor con la
--  HUELLA DE LA MÁQUINA (machine_id) como llave. Si el usuario borra la DB
--  local o reinstala, al volver a pedir la prueba el servidor devuelve la
--  MISMA fecha de inicio -> no puede reiniciar los 7 días.
-- =====================================================================

create table if not exists lic_trials (
  machine_id   text primary key,
  fecha_inicio timestamptz not null default now(),
  creado       timestamptz not null default now()
);

alter table lic_trials enable row level security; -- solo service_role (Edge Function)

-- Registra la máquina si es nueva (ON CONFLICT DO NOTHING fija el inicio la
-- primera vez) y devuelve el payload de la prueba para que la Edge Function
-- lo firme con Ed25519. Misma forma que LicensePayload en license.rs.
create or replace function fn_trial(p_machine_id text)
returns jsonb
language plpgsql
security definer
set search_path = public
as $$
declare
  v_inicio timestamptz;
  v_dias   int := 7;   -- duración de la prueba
begin
  insert into lic_trials (machine_id) values (p_machine_id)
    on conflict (machine_id) do nothing;

  select fecha_inicio into v_inicio from lic_trials where machine_id = p_machine_id;

  return jsonb_build_object(
    'ok', true,
    'payload', jsonb_build_object(
      'clave',            'TRIAL',
      'machine_id',       p_machine_id,
      'plan',             'Prueba',
      'fecha_activacion', v_inicio,
      'fecha_expiracion', v_inicio + make_interval(days => v_dias),
      'dias_gracia',      3
    )
  );
end;
$$;

revoke execute on function fn_trial(text) from public, anon, authenticated;
