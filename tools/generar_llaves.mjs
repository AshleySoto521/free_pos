// =====================================================================
//  generar_llaves.mjs  -  Genera el par de llaves Ed25519 (una sola vez)
//
//  Uso:
//    npm init -y
//    npm i @noble/ed25519@1.7.3
//    node generar_llaves.mjs
//
//  Salida:
//    - LLAVE PÚBLICA  -> pégala en license.rs (SERVER_PUBLIC_KEY_HEX)
//    - LLAVE PRIVADA  -> guárdala como Secret de Supabase (LICENSE_PRIVATE_KEY)
//
//  ⚠️ La privada NUNCA va en el .exe, ni en git, ni en el anon key.
//     Si se filtra, cualquiera puede fabricar licencias válidas.
// =====================================================================

import * as ed from "@noble/ed25519";

const toHex = (b) => Buffer.from(b).toString("hex");

const privada = ed.utils.randomPrivateKey();      // 32 bytes
const publica = await ed.getPublicKey(privada);   // 32 bytes

console.log("\n=================  LLAVES Ed25519  =================\n");
console.log("PÚBLICA  (license.rs -> SERVER_PUBLIC_KEY_HEX):");
console.log("  " + toHex(publica));
console.log("\nPRIVADA  (Supabase Secret -> LICENSE_PRIVATE_KEY):");
console.log("  " + toHex(privada));
console.log("\n⚠️  Resguarda la PRIVADA. No la subas a git ni al .exe.\n");
