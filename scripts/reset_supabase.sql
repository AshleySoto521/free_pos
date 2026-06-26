-- =====================================================================
--  RESET del licenciamiento en Supabase
--  Borra TODAS las tablas y funciones del licenciamiento para recrearlas
--  desde cero. ⚠️ Elimina las licencias y pruebas ya emitidas (solo para
--  ambiente de pruebas).
--
--  Cómo usar (Supabase -> SQL Editor), en este orden:
--    1) reset_supabase.sql      (este archivo)
--    2) licencias_supabase.sql
--    3) generar_licencias.sql
--    4) trial_supabase.sql
-- =====================================================================

-- Funciones primero (por si dependen de las tablas)
drop function if exists fn_activar_licencia(text, text, text, text, text);
drop function if exists fn_generar_licencia(text, text, text, int);
drop function if exists fn_nueva_clave();
drop function if exists fn_trial(text);

-- Tablas (CASCADE resuelve llaves foráneas e índices)
drop table if exists lic_log_validaciones cascade;
drop table if exists lic_activaciones cascade;
drop table if exists licencias cascade;
drop table if exists lic_planes cascade;
drop table if exists lic_compradores cascade;
drop table if exists lic_trials cascade;
