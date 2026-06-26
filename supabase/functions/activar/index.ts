// =====================================================================
//  Edge Function: activar
//  Recibe la clave + huella de máquina desde el .exe (Tauri), llama al
//  RPC fn_activar_licencia con el service_role y FIRMA el payload con
//  Ed25519. Devuelve { token } que el .exe verifica con la llave pública.
//
//  Desplegar:
//    supabase functions deploy activar --no-verify-jwt
//
//  Secrets requeridos (Project Settings -> Edge Functions -> Secrets):
//    LICENSE_PRIVATE_KEY   = <hex de la privada que imprimió generar_llaves.mjs>
//  (SUPABASE_URL y SUPABASE_SERVICE_ROLE_KEY ya existen por defecto)
// =====================================================================

/// <reference lib="deno.ns" />
import * as ed from "@noble/ed25519";
import { createClient } from "@supabase/supabase-js";

const SUPABASE_URL = Deno.env.get("SUPABASE_URL")!;
const SERVICE_KEY  = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!;
const PRIV_HEX     = Deno.env.get("LICENSE_PRIVATE_KEY")!;

// Cliente admin: omite RLS, puede llamar la función protegida.
const admin = createClient(SUPABASE_URL, SERVICE_KEY);

function hexToBytes(hex: string): Uint8Array {
  const a = new Uint8Array(hex.length / 2);
  for (let i = 0; i < a.length; i++) a[i] = parseInt(hex.substr(i * 2, 2), 16);
  return a;
}

// base64 estándar CON padding (igual que el engine STANDARD en Rust).
function b64(bytes: Uint8Array): string {
  let s = "";
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s);
}

function json(obj: unknown, status = 200): Response {
  return new Response(JSON.stringify(obj), {
    status,
    headers: { "content-type": "application/json" },
  });
}

Deno.serve(async (req) => {
  if (req.method !== "POST") return json({ error: "Method not allowed" }, 405);

  try {
    const body = await req.json();
    const { clave, machine_id, nombre_equipo, sistema_operativo, version_app } = body;

    if (!clave || !machine_id) return json({ error: "Faltan clave o machine_id" }, 400);

    // 1) Activación atómica en Postgres
    const { data, error } = await admin.rpc("fn_activar_licencia", {
      p_clave: clave,
      p_machine_id: machine_id,
      p_nombre_equipo: nombre_equipo ?? null,
      p_sistema_operativo: sistema_operativo ?? null,
      p_version_app: version_app ?? null,
    });

    if (error) return json({ error: error.message }, 500);
    if (!data?.ok) return json({ error: data?.error ?? "Rechazada" }, 403);

    // 2) Firmar el payload con la llave privada (Ed25519)
    const payloadBytes = new TextEncoder().encode(JSON.stringify(data.payload));
    const firma = await ed.sign(payloadBytes, hexToBytes(PRIV_HEX));

    // 3) token = "<payload_b64>.<firma_b64>"  (lo que espera license.rs)
    const token = `${b64(payloadBytes)}.${b64(firma)}`;
    return json({ token });
  } catch (e) {
    return json({ error: String(e) }, 500);
  }
});
