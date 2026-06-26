// =====================================================================
//  catalogos.ts  -  Catálogos de categorías por giro.
//  Los CSV viven en static/categorias/ y se descargan en runtime para que
//  el cliente elija con cuál arrancar (se importan con importar_categorias).
// =====================================================================

import { base } from '$app/paths';
import { parsearArchivo, encabezadosDe } from '$lib/xlsx';
import type { FilaCategoriaImport } from '$lib/api';

export interface CatalogoGiro {
	id: string;
	label: string;
	emoji: string;
	archivo: string;
}

/** Catálogos disponibles. El `archivo` debe existir en static/categorias/. */
export const catalogos: CatalogoGiro[] = [
	{ id: 'miscelanea', label: 'Miscelánea / Abarrotes', emoji: '🛒', archivo: 'categorias_miscelanea.csv' },
	{ id: 'farmacia', label: 'Farmacia', emoji: '💊', archivo: 'categorias_farmacia.csv' },
	{ id: 'panaderia', label: 'Panadería', emoji: '🥖', archivo: 'categorias_panaderia.csv' },
	{ id: 'papeleria', label: 'Papelería / Regalos', emoji: '✏️', archivo: 'categorias_papeleria_regalos.csv' },
	{ id: 'peluqueria', label: 'Peluquería / Estética', emoji: '💈', archivo: 'categorias_peluqueria_estetica.csv' },
	{ id: 'ferreteria', label: 'Ferretería / Tlapalería', emoji: '🔩', archivo: 'categorias_ferreteria_tlapaleria.csv' },
	{ id: 'jarceria', label: 'Jarcería / Limpieza', emoji: '🧹', archivo: 'categorias_jarceria_limpieza.csv' },
	{ id: 'refa_auto', label: 'Refaccionaria (Auto)', emoji: '🚗', archivo: 'categorias_refaccionaria_auto.csv' },
	{ id: 'refa_moto', label: 'Refaccionaria (Moto)', emoji: '🏍️', archivo: 'categorias_refaccionaria_moto.csv' },
	{ id: 'mascotas', label: 'Tienda de Mascotas', emoji: '🐾', archivo: 'categorias_tienda_mascotas.csv' }
];

/**
 * Descarga y parsea un catálogo de categorías. Reutiliza el parser de xlsx
 * (maneja CSV UTF-8 / Latin-1 y campos entre comillas con coma).
 */
export async function cargarCatalogo(c: CatalogoGiro): Promise<FilaCategoriaImport[]> {
	const resp = await fetch(`${base}/categorias/${c.archivo}`);
	if (!resp.ok) throw new Error(`No se pudo leer el catálogo (HTTP ${resp.status})`);
	const blob = await resp.blob();
	const file = new File([blob], c.archivo, { type: 'text/csv' });
	const filas = await parsearArchivo(file);
	if (!encabezadosDe(filas).includes('CATEGORIA')) {
		throw new Error('El catálogo no tiene la columna CATEGORIA');
	}
	return filas
		.filter((r) => r.CATEGORIA)
		.map((r) => ({ categoria: r.CATEGORIA, descripcion: r.DESCRIPCION || null }));
}
