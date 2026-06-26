// =====================================================================
//  ajustes.ts  -  Ajustes del negocio que cambian el comportamiento de la UI.
//  Por ahora: "modo de venta" (¿vende productos, servicios o ambos?).
//  Se guardan en la tabla Configuracion (llave-valor) y se cachean aquí.
// =====================================================================

import { writable } from 'svelte/store';
import { api } from '$lib/api';

export type ModoVenta = 'productos' | 'servicios' | 'ambos';

/** Qué vende el negocio. Default 'productos' hasta que se configure. */
export const modoVenta = writable<ModoVenta>('productos');

/** Giro del negocio (etiqueta libre, ej. 'Farmacia'). Solo informativo. */
export const giro = writable<string>('');

/** Carga los ajustes desde Configuracion. Idempotente; ignora errores. */
export async function cargarAjustes(): Promise<void> {
	try {
		const cfg = await api.listarConfig();
		const m = cfg.find((c) => c.clave === 'modo_venta')?.valor;
		if (m === 'productos' || m === 'servicios' || m === 'ambos') modoVenta.set(m);
		const g = cfg.find((c) => c.clave === 'giro')?.valor;
		if (g) giro.set(g);
	} catch {
		// Sin config aún (primer arranque): se quedan los valores por defecto.
	}
}

/** ¿La UI debe mostrar artículos tipo Producto (con inventario)? */
export function muestraProductos(m: ModoVenta): boolean {
	return m !== 'servicios';
}

/** ¿La UI debe mostrar artículos tipo Servicio (sin inventario)? */
export function muestraServicios(m: ModoVenta): boolean {
	return m !== 'productos';
}
