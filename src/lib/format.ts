// Formato de moneda. El símbolo y el código (divisa) son configurables y se
// fijan al iniciar la app con setMoneda(), según la divisa elegida. Se muestra
// el código junto al monto, p. ej. "$100.00 MXN" o "$10.00 USD".
let _simbolo = '$';
let _codigo = 'MXN';
const numero = new Intl.NumberFormat('es-MX', {
	minimumFractionDigits: 2,
	maximumFractionDigits: 2
});

/** Fija la divisa que se mostrará en TODOS los precios (símbolo + código). */
export function setMoneda(simbolo?: string | null, codigo?: string | null) {
	_simbolo = (simbolo && simbolo.trim()) || '$';
	_codigo = (codigo ?? '').trim();
}

export function pesos(n: number): string {
	const monto = `${_simbolo}${numero.format(n ?? 0)}`;
	return _codigo ? `${monto} ${_codigo}` : monto;
}

// ---------------------------------------------------------------------
//  Fechas: los timestamps de SQLite vienen en UTC sin zona
//  ('YYYY-MM-DD HH:MM:SS'). Aquí los interpretamos como UTC y los
//  mostramos en la zona/horario local de la máquina del usuario.
// ---------------------------------------------------------------------

function aLocal(s: string): Date {
	let t = s.trim();
	if (!t.includes('T')) t = t.replace(' ', 'T');
	// Si no trae zona (Z u offset), es UTC -> se la agregamos.
	if (!/[Zz]$|[+-]\d\d:?\d\d$/.test(t)) t += 'Z';
	return new Date(t);
}

/** Fecha + hora en formato local corto: "26/06/2026, 09:05". */
export function fechaHora(s: string | null | undefined): string {
	if (!s) return '';
	return aLocal(s).toLocaleString('es-MX', {
		day: '2-digit',
		month: '2-digit',
		year: 'numeric',
		hour: '2-digit',
		minute: '2-digit'
	});
}

/** Solo la hora local: "09:05". */
export function horaLocal(s: string | null | undefined): string {
	if (!s) return '';
	return aLocal(s).toLocaleTimeString('es-MX', { hour: '2-digit', minute: '2-digit' });
}
