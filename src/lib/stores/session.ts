// =====================================================================
//  session.ts  -  Estado de sesión del usuario y de la licencia.
// =====================================================================

import { writable } from 'svelte/store';
import { api, type LicenseStatus, type Usuario } from '$lib/api';

/** Usuario con sesión activa, o null si nadie ha iniciado sesión. */
export const session = writable<Usuario | null>(null);

/** Última info de licencia conocida (para banners de "te quedan X días"). */
export const licencia = writable<LicenseStatus | null>(null);

export function setSession(usuario: Usuario) {
	session.set(usuario);
}

export function logout() {
	// Limpia también la sesión del backend (RBAC). Fire-and-forget.
	api.cerrarSesion().catch(() => {});
	session.set(null);
}

/** True si el usuario en sesión es Administrador. */
export function esAdministrador(u: Usuario | null): boolean {
	return u?.rol === 'Administrador';
}

/** True solo cuando la app corre dentro de la ventana de Tauri. */
export function enTauri(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
