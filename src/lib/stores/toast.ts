// =====================================================================
//  toast.ts  -  Notificaciones breves (esquina inferior derecha).
// =====================================================================

import { writable } from 'svelte/store';

export type Toast = { id: number; msg: string; tipo: 'ok' | 'error' };

export const toasts = writable<Toast[]>([]);

let contador = 0;

export function toast(msg: string, tipo: 'ok' | 'error' = 'ok') {
	const id = ++contador;
	toasts.update((t) => [...t, { id, msg, tipo }]);
	setTimeout(() => {
		toasts.update((t) => t.filter((x) => x.id !== id));
	}, 2800);
}
