// =====================================================================
//  pdf.ts  -  Generar PDF con jsPDF + autotable.
// =====================================================================

import { jsPDF } from 'jspdf';
import autoTable from 'jspdf-autotable';

type Celda = string | number;

export function exportarPDF(opts: {
	nombre: string;
	titulo: string;
	subtitulo?: string;
	encabezados: string[];
	filas: Celda[][];
}) {
	const doc = new jsPDF();

	doc.setFontSize(14);
	doc.text(opts.titulo, 14, 16);

	let inicio = 22;
	if (opts.subtitulo) {
		doc.setFontSize(10);
		doc.setTextColor(110);
		doc.text(opts.subtitulo, 14, 22);
		doc.setTextColor(0);
		inicio = 28;
	}

	autoTable(doc, {
		startY: inicio,
		head: [opts.encabezados],
		body: opts.filas.map((f) => f.map((c) => String(c))),
		styles: { fontSize: 8, cellPadding: 2 },
		headStyles: { fillColor: [79, 70, 229] }
	});

	doc.save(`${opts.nombre}.pdf`);
}
