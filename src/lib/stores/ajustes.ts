// =====================================================================
//  ajustes.ts  -  Ajustes del negocio que cambian el comportamiento de la UI.
//  Por ahora: "modo de venta" (¿vende productos, servicios o ambos?).
//  Se guardan en la tabla Configuracion (llave-valor) y se cachean aquí.
// =====================================================================

import { writable } from 'svelte/store';
import { api } from '$lib/api';
import { setMoneda } from '$lib/format';

export type ModoVenta = 'productos' | 'servicios' | 'ambos';

/** Qué vende el negocio. Default 'productos' hasta que se configure. */
export const modoVenta = writable<ModoVenta>('productos');

/** Giro del negocio (etiqueta libre, ej. 'Farmacia'). Solo informativo. */
export const giro = writable<string>('');

/** Divisa activa (símbolo + código) para mostrar junto a los precios. */
export const moneda = writable<{ simbolo: string; codigo: string }>({ simbolo: '$', codigo: 'MXN' });

/** Aplica la divisa al formateador y al store. */
function aplicarMoneda(simbolo: string | null | undefined, codigo: string | null | undefined) {
	setMoneda(simbolo, codigo);
	moneda.set({ simbolo: simbolo?.trim() || '$', codigo: codigo?.trim() || '' });
}

/** Carga los ajustes desde Configuracion. Idempotente; ignora errores. */
export async function cargarAjustes(): Promise<void> {
	try {
		const cfg = await api.listarConfig();
		const m = cfg.find((c) => c.clave === 'modo_venta')?.valor;
		if (m === 'productos' || m === 'servicios' || m === 'ambos') modoVenta.set(m);
		const g = cfg.find((c) => c.clave === 'giro')?.valor;
		if (g) giro.set(g);

		// Divisa: primero la elegida en config; si no, la principal del catálogo.
		const cod = cfg.find((c) => c.clave === 'moneda_codigo')?.valor;
		const simb = cfg.find((c) => c.clave === 'moneda_simbolo')?.valor;
		if (cod || simb) {
			aplicarMoneda(simb, cod);
		} else {
			const ms = await api.listarMonedas();
			const ppal = ms.find((m) => m.esPrincipal) ?? ms[0];
			if (ppal) aplicarMoneda(ppal.simbolo, ppal.codigo);
		}
	} catch {
		// Sin config aún (primer arranque): se quedan los valores por defecto.
	}
}

/** Cambia la divisa en caliente (tras guardarla en Configuración/onboarding). */
export function fijarMoneda(simbolo: string | null | undefined, codigo: string | null | undefined) {
	aplicarMoneda(simbolo, codigo);
}

/** ¿La UI debe mostrar artículos tipo Producto (con inventario)? */
export function muestraProductos(m: ModoVenta): boolean {
	return m !== 'servicios';
}

/** ¿La UI debe mostrar artículos tipo Servicio (sin inventario)? */
export function muestraServicios(m: ModoVenta): boolean {
	return m !== 'productos';
}
