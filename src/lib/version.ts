// =====================================================================
//  Versión del software (FUENTE ÚNICA para la UI: footer y Guía).
//  Al publicar una versión nueva, sube este número y haz que coincida con:
//    - package.json            ("version")
//    - src-tauri/tauri.conf.json ("version")  → versión del instalador/.exe
//    - src-tauri/Cargo.toml     ("version")
//  Formato SemVer: MAYOR.MENOR.PARCHE
//    MAYOR  = cambios grandes/incompatibles
//    MENOR  = funciones nuevas compatibles
//    PARCHE = correcciones
// =====================================================================
export const APP_VERSION = '1.0.0';
