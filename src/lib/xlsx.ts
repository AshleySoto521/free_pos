// =====================================================================
//  xlsx.ts  -  Exportar e importar XLSX/CSV con SheetJS.
//  La importación normaliza encabezados (TRIM + MAYÚSCULAS) para validarlos
//  contra los de la tabla; la limpieza autoritativa la hace el backend.
// =====================================================================

import * as XLSX from 'xlsx';

type Celda = string | number | null;

/** Genera y descarga un .xlsx con encabezados + filas. */
export function exportarXLSX(nombre: string, encabezados: string[], filas: Celda[][]) {
	const ws = XLSX.utils.aoa_to_sheet([encabezados, ...filas]);
	const wb = XLSX.utils.book_new();
	XLSX.utils.book_append_sheet(wb, ws, 'Datos');
	XLSX.writeFile(wb, `${nombre}.xlsx`);
}

/**
 * Descarga una plantilla .csv vacía con los encabezados (y una fila de ejemplo
 * opcional). Lleva BOM UTF-8 para que Excel muestre bien los acentos.
 */
export function descargarPlantillaCSV(
	nombre: string,
	encabezados: string[],
	ejemplo: string[] = []
) {
	const escapar = (v: string) => {
		const s = String(v ?? '');
		return /[",\n\r;]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
	};
	const lineas = [encabezados.map(escapar).join(',')];
	if (ejemplo.length) lineas.push(ejemplo.map(escapar).join(','));
	const contenido = '﻿' + lineas.join('\r\n') + '\r\n';
	const blob = new Blob([contenido], { type: 'text/csv;charset=utf-8;' });
	const url = URL.createObjectURL(blob);
	const a = document.createElement('a');
	a.href = url;
	a.download = `plantilla_${nombre}.csv`;
	a.click();
	URL.revokeObjectURL(url);
}

/**
 * Decodifica bytes a texto: intenta UTF-8 estricto y, si el archivo no es
 * UTF-8 válido (típico de CSV guardados en Excel/Bloc de notas como Latin-1),
 * cae a Windows-1252. Así los acentos se leen bien en cualquier caso.
 */
function decodificar(bytes: Uint8Array): string {
	try {
		return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
	} catch {
		return new TextDecoder('windows-1252').decode(bytes);
	}
}

/**
 * Lee un archivo .xlsx o .csv y devuelve las filas como objetos cuyas llaves
 * son los encabezados normalizados (sin espacios extra y en MAYÚSCULAS).
 * Los .csv se decodifican detectando la codificación (UTF-8 o Latin-1).
 */
export async function parsearArchivo(file: File): Promise<Record<string, string>[]> {
	const buf = await file.arrayBuffer();
	const esCSV = file.name.toLowerCase().endsWith('.csv');
	const wb = esCSV
		? XLSX.read(decodificar(new Uint8Array(buf)), { type: 'string' })
		: XLSX.read(buf, { type: 'array' });
	const ws = wb.Sheets[wb.SheetNames[0]];
	if (!ws) return [];
	const crudas = XLSX.utils.sheet_to_json<Record<string, unknown>>(ws, { defval: '', raw: false });

	return crudas.map((fila) => {
		const limpia: Record<string, string> = {};
		for (const llave of Object.keys(fila)) {
			const k = llave.trim().toUpperCase();
			limpia[k] = String(fila[llave] ?? '').trim();
		}
		return limpia;
	});
}

/** Devuelve los encabezados (normalizados) presentes en las filas parseadas. */
export function encabezadosDe(filas: Record<string, string>[]): string[] {
	return filas.length ? Object.keys(filas[0]) : [];
}
