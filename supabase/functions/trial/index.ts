// =====================================================================
//  Edge Function: trial
//  Recibe la huella de máquina (machine_id), registra/consulta la prueba en
//  lic_trials (vía fn_trial) y FIRMA el payload con Ed25519. El .exe verifica
//  la firma con la llave pública embebida. Anti-reset: el inicio se ancla por
//  machine_id en el servidor.
//
//  Desplegar:  supabase functions deploy trial --no-verify-jwt
//  Usa el mismo Secret LICENSE_PRIVATE_KEY que la función `activar`.
// =====================================================================

/// <reference lib="deno.ns" />
import * as ed from "@noble/ed25519";
import { createClient } from "@supabase/supabase-js";

const SUPABASE_URL = Deno.env.get("SUPABASE_URL")!;
const SERVICE_KEY = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!;
const PRIV_HEX = Deno.env.get("LICENSE_PRIVATE_KEY")!;

const admin = createClient(SUPABASE_URL, SERVICE_KEY);

function hexToBytes(hex: string): Uint8Array {
	const a = new Uint8Array(hex.length / 2);
	for (let i = 0; i < a.length; i++) a[i] = parseInt(hex.substr(i * 2, 2), 16);
	return a;
}

function b64(bytes: Uint8Array): string {
	let s = "";
	for (const b of bytes) s += String.fromCharCode(b);
	return btoa(s);
}

function json(obj: unknown, status = 200): Response {
	return new Response(JSON.stringify(obj), {
		status,
		headers: { "content-type": "application/json" }
	});
}

Deno.serve(async (req) => {
	if (req.method !== "POST") return json({ error: "Method not allowed" }, 405);
	try {
		const { machine_id } = await req.json();
		if (!machine_id) return json({ error: "Falta machine_id" }, 400);

		const { data, error } = await admin.rpc("fn_trial", { p_machine_id: machine_id });
		if (error) return json({ error: error.message }, 500);
		if (!data?.ok) return json({ error: data?.error ?? "No disponible" }, 403);

		const payloadBytes = new TextEncoder().encode(JSON.stringify(data.payload));
		const firma = await ed.sign(payloadBytes, hexToBytes(PRIV_HEX));
		const token = `${b64(payloadBytes)}.${b64(firma)}`;
		return json({ token });
	} catch (e) {
		return json({ error: String(e) }, 500);
	}
});
