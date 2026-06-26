-- =====================================================================
--  GENERADOR DE LICENCIAS  (correr en Supabase -> SQL Editor)
--  Requiere haber corrido antes licencias_supabase.sql
--
--  Modelo:
--   - El PERIODO sale del PLAN (dias_vigencia) y se cuenta desde la
--     PRIMERA activación. Si quieres un periodo distinto, crea otro plan.
--   - El NÚMERO DE DISPOSITIVOS sale del plan, pero se puede sobreescribir
--     por licencia con el parámetro p_max_activaciones.
-- =====================================================================

-- ---------------------------------------------------------------------
--  Generador de claves con formato POS-XXXX-XXXX-XXXX-XXXX
-- ---------------------------------------------------------------------
create or replace function fn_nueva_clave()
returns text
language sql
volatile
as $$
  select 'POS-' || string_agg(g, '-')
  from (
    select upper(substr(encode(gen_random_bytes(8), 'hex'), p, 4)) as g
    from generate_series(1, 13, 4) as p
  ) t;
$$;

-- ---------------------------------------------------------------------
--  Crear una licencia. Devuelve la clave generada.
--    p_plan             : 'Mensual' / 'Anual' / 'Perpetua' (debe existir en lic_planes)
--    p_email, p_nombre  : comprador (opcional; se crea/reutiliza por email)
--    p_max_activaciones : override de dispositivos (null = el del plan)
-- ---------------------------------------------------------------------
create or replace function fn_generar_licencia(
  p_plan             text,
  p_email            text default null,
  p_nombre           text default null,
  p_max_activaciones int  default null
)
returns text
language plpgsql
security definer
set search_path = public
as $$
declare
  v_plan_id      bigint;
  v_comprador_id bigint;
  v_clave        text;
begin
  select id into v_plan_id from lic_planes where plan = p_plan and activo limit 1;
  if v_plan_id is null then
    raise exception 'El plan "%" no existe o está inactivo', p_plan;
  end if;

  -- Comprador opcional (se reutiliza si el email ya existe)
  if p_email is not null then
    insert into lic_compradores (nombre, email)
      values (coalesce(p_nombre, p_email), p_email)
    on conflict (email) do update
      set nombre = coalesce(excluded.nombre, lic_compradores.nombre)
    returning id into v_comprador_id;
  end if;

  -- Clave única
  loop
    v_clave := fn_nueva_clave();
    exit when not exists (select 1 from licencias where clave = v_clave);
  end loop;

  insert into licencias (clave, comprador_id, plan_id, max_activaciones, estado)
    values (v_clave, v_comprador_id, v_plan_id, p_max_activaciones, 'Emitida');

  return v_clave;
end;
$$;

-- Solo tú (service_role / SQL Editor). Que no se llame desde la API pública.
revoke execute on function fn_generar_licencia(text,text,text,int) from public, anon, authenticated;


-- =====================================================================
--  EJEMPLOS DE USO  (descomenta y corre lo que necesites)
-- =====================================================================

-- 1) Licencia ANUAL, 1 dispositivo, ligada a un cliente:
-- select fn_generar_licencia('Anual', 'rosa@correo.com', 'Tienda Doña Rosa');

-- 2) Licencia PERPETUA para 3 cajas (override de dispositivos):
-- select fn_generar_licencia('Perpetua', 'dueno@super.com', 'Super El Ahorro', 3);

-- 3) Generar 10 claves MENSUALES sueltas (sin cliente) para vender:
-- select fn_generar_licencia('Mensual') from generate_series(1, 10);


-- =====================================================================
--  CONSULTAS DE ADMINISTRACIÓN
-- =====================================================================

-- Ver todo lo emitido (con dispositivos efectivos y uso actual):
-- select
--   l.clave,
--   p.plan,
--   coalesce(l.max_activaciones, p.max_activaciones) as dispositivos_max,
--   (select count(*) from lic_activaciones a
--      where a.licencia_id = l.id and a.estado = 'Activa')  as dispositivos_en_uso,
--   p.dias_vigencia,
--   l.estado,
--   l.fecha_expiracion,
--   c.email
-- from licencias l
-- join lic_planes p on p.id = l.plan_id
-- left join lic_compradores c on c.id = l.comprador_id
-- order by l.fecha_emision desc;

-- Suspender / reactivar una licencia (el .exe deja de validar al revalidar):
-- update licencias set estado = 'Suspendida' where clave = 'POS-....';
-- update licencias set estado = 'Activa'      where clave = 'POS-....';

-- Liberar un dispositivo (para que el cliente pueda activar en otra PC):
-- update lic_activaciones set estado = 'Revocada'
--   where licencia_id = (select id from licencias where clave = 'POS-....')
--     and machine_id  = '<machine_id a liberar>';

-- Ver los dispositivos de una licencia:
-- select a.machine_id, a.nombre_equipo, a.fecha_activacion, a.ultimo_chequeo, a.estado
-- from lic_activaciones a
-- join licencias l on l.id = a.licencia_id
-- where l.clave = 'POS-....';
