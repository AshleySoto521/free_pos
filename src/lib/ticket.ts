// =====================================================================
//  ticket.ts  -  Genera el ticket de venta en PDF con ancho de impresora
//  térmica (80mm). Sirve para imprimir o como respaldo si no hay impresora.
// =====================================================================

import { jsPDF } from 'jspdf';
import { pesos } from './format';

export interface TicketItem {
	nombre: string;
	cantidad: number;
	precio: number;
}

export interface TicketData {
	folio: string;
	fecha: string; // ya formateada en hora local
	cajero: string;
	cliente: string;
	items: TicketItem[];
	total: number;
	metodo: string;
	pagoCon: number | null;
	cambio: number | null;
}

export function generarTicketPDF(t: TicketData, negocio: Record<string, string>) {
	const W = 80; // ancho de papel térmico (mm)
	const m = 5; // margen
	const alto = 78 + t.items.length * 9; // altura estimada (papel continuo)
	const doc = new jsPDF({ unit: 'mm', format: [W, alto] });
	let y = 8;

	const centro = (txt: string, size = 8, bold = false) => {
		doc.setFont('helvetica', bold ? 'bold' : 'normal');
		doc.setFontSize(size);
		doc.text(txt, W / 2, y, { align: 'center' });
	};
	const izq = (txt: string, size = 8, bold = false) => {
		doc.setFont('helvetica', bold ? 'bold' : 'normal');
		doc.setFontSize(size);
		doc.text(txt, m, y);
	};
	const der = (txt: string, size = 8, bold = false) => {
		doc.setFont('helvetica', bold ? 'bold' : 'normal');
		doc.setFontSize(size);
		doc.text(txt, W - m, y, { align: 'right' });
	};
	const linea = () => {
		doc.setLineWidth(0.1);
		doc.line(m, y, W - m, y);
	};

	// Cabecera del negocio
	centro(negocio.NombreTienda || 'Mi Negocio', 11, true);
	y += 5;
	if (negocio.Direccion) {
		centro(negocio.Direccion, 7);
		y += 3.5;
	}
	if (negocio.Telefono) {
		centro('Tel. ' + negocio.Telefono, 7);
		y += 3.5;
	}
	y += 1;
	linea();
	y += 4;

	izq('Folio: ' + t.folio, 7);
	der(t.fecha, 7);
	y += 4;
	izq('Cajero: ' + t.cajero, 7);
	y += 4;
	izq('Cliente: ' + t.cliente, 7);
	y += 3;
	linea();
	y += 4;

	// Productos
	for (const it of t.items) {
		izq(it.nombre, 8);
		y += 4;
		izq('   ' + it.cantidad + ' x ' + pesos(it.precio), 7);
		der(pesos(it.precio * it.cantidad), 8);
		y += 5;
	}
	linea();
	y += 5;

	izq('TOTAL', 11, true);
	der(pesos(t.total), 11, true);
	y += 6;
	izq(t.metodo, 8);
	y += 4;
	if (t.pagoCon != null) {
		izq('Pagó: ' + pesos(t.pagoCon), 7);
		der('Cambio: ' + pesos(t.cambio ?? 0), 7);
		y += 4;
	}
	y += 1;
	linea();
	y += 5;

	if (negocio.PieTicket) {
		centro(negocio.PieTicket, 7);
		y += 4;
	}
	centro('¡Gracias por su compra!', 8);

	doc.save(`ticket_${t.folio}.pdf`);
}
