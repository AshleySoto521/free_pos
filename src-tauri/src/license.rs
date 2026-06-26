// =====================================================================
//  license.rs  -  Lógica PURA de licencias (Tauri + Rust)
//
//  Aquí solo viven las funciones de verificación/firma y la llamada de red.
//  Los comandos Tauri (que tocan la DB) están en commands.rs.
//
//  IDEA CLAVE:
//   - El SERVIDOR firma un token con su LLAVE PRIVADA (Ed25519).
//   - El .exe trae embebida solo la LLAVE PÚBLICA y verifica la firma.
//   - El usuario NO puede falsificar un token válido sin la llave privada.
// =====================================================================

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::{DateTime, Duration, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------
//  CONFIGURACIÓN
// ---------------------------------------------------------------------

// Los valores del despliegue (URL de Supabase, anon key, llave pública) se
// INYECTAN en tiempo de compilación desde variables de entorno, para no
// guardarlos en el código (ni subirlos a git). Se definen en
// `src-tauri/.cargo/config.toml` (archivo gitignoreado; copia el .example).
// Si no están definidas, el binario corre en MODO LOCAL (trial sin servidor).

/// Llave PÚBLICA del servidor (32 bytes en hex). La privada vive SOLO en
/// Supabase (Secret). La pública no es secreta, pero se inyecta junto al resto.
const SERVER_PUBLIC_KEY_HEX: &str = match option_env!("FREEPOS_LICENSE_PUBLIC_KEY") {
    Some(v) => v,
    None => "",
};

/// URL base del proyecto Supabase (ej. https://xxxxx.supabase.co).
const SUPABASE_URL: &str = match option_env!("FREEPOS_SUPABASE_URL") {
    Some(v) => v,
    None => "",
};

/// anon/public key de Supabase. Es segura para el cliente; se manda como gate
/// ligero anti-spam cuando las Edge Functions tienen "Verify JWT" activado.
const SUPABASE_ANON_KEY: &str = match option_env!("FREEPOS_SUPABASE_ANON_KEY") {
    Some(v) => v,
    None => "",
};

/// Endpoint de activación (Edge Function).
fn activation_url() -> String {
    format!("{SUPABASE_URL}/functions/v1/activar")
}

/// Endpoint de la prueba (trial) anclada por máquina.
fn trial_url() -> String {
    format!("{SUPABASE_URL}/functions/v1/trial")
}

/// True cuando hay servidor configurado (URL + llave pública real). Mientras
/// no lo esté, el trial funciona en modo local (desarrollo / repo sin secretos).
pub fn servidor_configurado() -> bool {
    !SUPABASE_URL.is_empty() && SERVER_PUBLIC_KEY_HEX.chars().any(|c| c != '0')
}

// ---------------------------------------------------------------------
//  MODELOS
// ---------------------------------------------------------------------

/// Lo que el servidor firma. Coincide con el payload de fn_activar_licencia.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LicensePayload {
    pub clave: String,
    pub machine_id: String,
    pub plan: String,
    pub fecha_activacion: DateTime<Utc>,
    pub fecha_expiracion: Option<DateTime<Utc>>, // None = perpetua
    pub dias_gracia: i64,
}

/// Respuesta de la Edge Function: { "token": "<payload_b64>.<firma_b64>" }
#[derive(Deserialize)]
struct SignedToken {
    token: String,
}

/// Resultado que se devuelve al frontend de Svelte.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LicenseStatus {
    pub valida: bool,
    pub estado: String, // Activa/Expirada/SinActivar/GraciaAgotada/Manipulada/Error
    pub mensaje: String,
    pub fecha_expiracion: Option<DateTime<Utc>>,
    pub dias_restantes: Option<i64>,
}

impl LicenseStatus {
    pub fn invalida(estado: &str, msg: &str) -> Self {
        LicenseStatus {
            valida: false,
            estado: estado.into(),
            mensaje: msg.into(),
            fecha_expiracion: None,
            dias_restantes: None,
        }
    }

    pub fn sin_activar() -> Self {
        LicenseStatus::invalida("SinActivar", "Ingresa tu clave de producto para activar.")
    }
}

// ---------------------------------------------------------------------
//  HUELLA DE MÁQUINA
// ---------------------------------------------------------------------

/// ID estable de esta PC, hasheado (no exponemos el ID crudo).
pub fn get_machine_id() -> Result<String, String> {
    let raw = machine_uid::get().map_err(|e| format!("No se pudo leer el ID de máquina: {e}"))?;
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hasher.update(b"FreePOS-v1"); // sal fija propia de la app
    Ok(format!("{:x}", hasher.finalize()))
}

// ---------------------------------------------------------------------
//  VERIFICACIÓN DE FIRMA (offline)
// ---------------------------------------------------------------------

fn load_public_key() -> Result<VerifyingKey, String> {
    let bytes = hex_to_bytes(SERVER_PUBLIC_KEY_HEX)?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "La llave pública no mide 32 bytes".to_string())?;
    VerifyingKey::from_bytes(&arr).map_err(|e| format!("Llave pública inválida: {e}"))
}

/// Verifica la firma del token y devuelve el payload si es auténtico.
/// Aquí se cae cualquier token falsificado por el usuario.
pub fn verify_token_signature(token: &str) -> Result<LicensePayload, String> {
    let (payload_b64, sig_b64) = token.split_once('.').ok_or("Formato de token inválido")?;

    let payload_bytes = B64
        .decode(payload_b64)
        .map_err(|_| "Payload no es base64 válido")?;
    let sig_bytes = B64.decode(sig_b64).map_err(|_| "Firma no es base64 válida")?;

    let sig = Signature::from_slice(&sig_bytes).map_err(|_| "Firma con tamaño inválido")?;
    let vk = load_public_key()?;

    vk.verify(&payload_bytes, &sig)
        .map_err(|_| "Firma inválida: el token fue manipulado o no es de este servidor")?;

    serde_json::from_slice(&payload_bytes).map_err(|_| "Payload ilegible".to_string())
}

// ---------------------------------------------------------------------
//  VALIDACIÓN COMPLETA (firma + máquina + expiración + gracia)
// ---------------------------------------------------------------------

/// Evalúa un token ya guardado. `ultimo_chequeo_online` viene de LicenciaLocal.
pub fn evaluate_license(
    token: &str,
    ultimo_chequeo_online: Option<DateTime<Utc>>,
) -> LicenseStatus {
    // 1) Firma auténtica
    let payload = match verify_token_signature(token) {
        Ok(p) => p,
        Err(e) => return LicenseStatus::invalida("Manipulada", &e),
    };

    // 2) ¿El token es para ESTA máquina?
    match get_machine_id() {
        Ok(mid) if mid == payload.machine_id => {}
        Ok(_) => {
            return LicenseStatus::invalida(
                "SinActivar",
                "Esta licencia fue activada en otra computadora.",
            )
        }
        Err(e) => return LicenseStatus::invalida("Error", &e),
    }

    let ahora = Utc::now();

    // 3) ¿Expiró? (suscripción)
    if let Some(exp) = payload.fecha_expiracion {
        if ahora > exp {
            return LicenseStatus {
                valida: false,
                estado: "Expirada".into(),
                mensaje: "Tu licencia venció. Renueva para seguir usando el POS.".into(),
                fecha_expiracion: Some(exp),
                dias_restantes: Some(0),
            };
        }
    }

    // 4) Periodo de gracia offline
    if let Some(ultimo) = ultimo_chequeo_online {
        let limite = ultimo + Duration::days(payload.dias_gracia);
        if ahora > limite {
            return LicenseStatus::invalida(
                "GraciaAgotada",
                "Conéctate a internet para revalidar tu licencia.",
            );
        }
    }

    // 5) OK — distinguimos prueba de licencia de pago para la UI.
    let dias_restantes = payload.fecha_expiracion.map(|exp| (exp - ahora).num_days());
    let es_prueba = payload.plan.eq_ignore_ascii_case("Prueba");
    LicenseStatus {
        valida: true,
        estado: if es_prueba { "Prueba" } else { "Activa" }.into(),
        mensaje: if es_prueba {
            "Versión de prueba.".into()
        } else {
            "Licencia activa.".into()
        },
        fecha_expiracion: payload.fecha_expiracion,
        dias_restantes,
    }
}

// ---------------------------------------------------------------------
//  LLAMADA DE RED (activación / revalidación contra el servidor)
// ---------------------------------------------------------------------

/// Pide el token firmado al servidor para esta máquina. Devuelve el token
/// (ya verificado) listo para guardar en LicenciaLocal.
pub async fn solicitar_token(clave: &str) -> Result<String, String> {
    let machine_id = get_machine_id()?;

    let cuerpo = serde_json::json!({
        "clave": clave,
        "machine_id": machine_id,
        "nombre_equipo": hostname(),
        "sistema_operativo": std::env::consts::OS,
        "version_app": env!("CARGO_PKG_VERSION"),
    });

    let resp = reqwest::Client::new()
        .post(activation_url())
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", SUPABASE_ANON_KEY))
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| format!("No se pudo contactar al servidor: {e}"))?;

    if !resp.status().is_success() {
        // La Edge Function manda { "error": "..." } cuando rechaza.
        let status = resp.status();
        let detalle = resp
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
            .unwrap_or_else(|| status.to_string());
        return Err(format!("Activación rechazada: {detalle}"));
    }

    let token: SignedToken = resp
        .json()
        .await
        .map_err(|e| format!("Respuesta del servidor ilegible: {e}"))?;

    // Verifica de una vez que el token recibido sea auténtico.
    verify_token_signature(&token.token)?;
    Ok(token.token)
}

/// Pide al servidor el token firmado de la PRUEBA para esta máquina.
/// El servidor ancla la fecha de inicio por machine_id (anti-reset).
pub async fn solicitar_trial() -> Result<String, String> {
    let machine_id = get_machine_id()?;
    let cuerpo = serde_json::json!({ "machine_id": machine_id });

    let resp = reqwest::Client::new()
        .post(trial_url())
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", SUPABASE_ANON_KEY))
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| format!("No se pudo contactar al servidor: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("No se pudo iniciar la prueba: {}", resp.status()));
    }

    let token: SignedToken = resp
        .json()
        .await
        .map_err(|e| format!("Respuesta del servidor ilegible: {e}"))?;
    verify_token_signature(&token.token)?;
    Ok(token.token)
}

// ---------------------------------------------------------------------
//  HELPERS
// ---------------------------------------------------------------------

fn hostname() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Desconocido".into())
}

fn hex_to_bytes(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("Hex de longitud impar".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}
