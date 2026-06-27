<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import {
		api,
		type Producto,
		type MetodoPago,
		type Cliente,
		type Corte,
		type VentaResultado
	} from '$lib/api';
	import { session } from '$lib/stores/session';
	import { pesos, fechaHora } from '$lib/format';
	import { generarTicketPDF, type TicketData } from '$lib/ticket';
	import { toast } from '$lib/stores/toast';
	import ContadorMonedas from '$lib/components/ContadorMonedas.svelte';

	type Linea = { producto: Producto; cantidad: number };

	// --- Datos ---
	let productos = $state<Producto[]>([]);
	let metodos = $state<MetodoPago[]>([]);
	let clientes = $state<Cliente[]>([]);
	let corte = $state<Corte | null>(null);
	let cargando = $state(true);
	let negocio = $state<Record<string, string>>({});

	// Reloj en vivo: fecha y hora actuales a la vista del cajero.
	let ahora = $state(new Date());
	const reloj = $derived(
		ahora.toLocaleString('es-MX', {
			weekday: 'short',
			day: '2-digit',
			month: '2-digit',
			year: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit'
		})
	);
	$effect(() => {
		const id = setInterval(() => (ahora = new Date()), 1000);
		return () => clearInterval(id);
	});

	// Abrir caja desde aquí (no se puede vender sin caja abierta)
	let montoInicialCaja = $state<number | null>(0);
	let abriendoCaja = $state(false);
	let errorCaja = $state('');

	// --- Carrito ---
	let carrito = $state<Linea[]>([]);
	let busqueda = $state('');
	let codigo = $state('');
	let inputCodigo = $state<HTMLInputElement | null>(null);
	let aviso = $state('');

	const total = $derived(carrito.reduce((s, l) => s + l.producto.precioUnitario * l.cantidad, 0));
	const totalArticulos = $derived(carrito.reduce((s, l) => s + l.cantidad, 0));

	// Qué vende el negocio (productos/servicios/ambos) — filtra el catálogo.
	const modo = $derived(negocio['modo_venta'] || 'productos');
	const visiblesPorModo = $derived(
		productos.filter((p) =>
			modo === 'ambos'
				? true
				: modo === 'servicios'
					? p.tipo === 'Servicio'
					: p.tipo !== 'Servicio'
		)
	);
	const productosFiltrados = $derived(
		busqueda.trim()
			? visiblesPorModo.filter((p) =>
					(p.producto + ' ' + (p.codigoBarras ?? '')).toLowerCase().includes(busqueda.toLowerCase())
				)
			: visiblesPorModo
	);

	// --- Cobro (modal) ---
	let cobrando = $state(false);
	let idMetodo = $state<number | null>(null);
	let referencia = $state('');
	let pagaConTxt = $state('');
	let idCliente = $state<number | null>(null);
	let procesando = $state(false);
	let errorVenta = $state('');
	let ticket = $state<VentaResultado | null>(null);
	let ticketData = $state<TicketData | null>(null);

	const metodoSel = $derived(metodos.find((m) => m.idMetodoPago === idMetodo) ?? null);
	const esFiado = $derived(metodoSel?.metodoPago.toLowerCase() === 'fiado');
	const pagaCon = $derived(pagaConTxt.trim() === '' ? null : Number(pagaConTxt));
	const cambio = $derived(pagaCon != null && !esFiado ? pagaCon - total : null);

	onMount(async () => {
		try {
			const [pr, me, cl, cfg] = await Promise.all([
				api.listarProductos(),
				api.listarMetodosPago(),
				api.listarClientes(),
				api.listarConfig()
			]);
			productos = pr;
			metodos = me;
			clientes = cl;
			negocio = Object.fromEntries(cfg.map((c) => [c.clave, c.valor]));
			if ($session) corte = await api.corteAbierto($session.idUsuario);
		} finally {
			cargando = false;
			await tick();
			inputCodigo?.focus();
		}
	});

	async function abrirCaja() {
		if (!$session) return;
		abriendoCaja = true;
		errorCaja = '';
		try {
			await api.abrirCorte($session.idUsuario, montoInicialCaja ?? 0);
			corte = await api.corteAbierto($session.idUsuario);
			await tick();
			inputCodigo?.focus();
		} catch (e) {
			errorCaja = String(e);
		} finally {
			abriendoCaja = false;
		}
	}

	function agregar(p: Producto) {
		const existente = carrito.find((l) => l.producto.idProducto === p.idProducto);
		if (existente) existente.cantidad += 1;
		else carrito.push({ producto: p, cantidad: 1 });
		aviso = '';
	}

	async function porCodigo(e: KeyboardEvent) {
		if (e.key !== 'Enter') return;
		const c = codigo.trim();
		if (!c) return;
		codigo = '';
		try {
			const p = await api.buscarProductoPorCodigo(c);
			if (p) agregar(p);
			else aviso = `Sin coincidencia para "${c}"`;
		} catch (err) {
			aviso = String(err);
		}
	}

	function cambiarCantidad(l: Linea, delta: number) {
		l.cantidad = Math.max(0, +(l.cantidad + delta).toFixed(3));
		if (l.cantidad === 0) quitar(l);
	}

	function quitar(l: Linea) {
		carrito = carrito.filter((x) => x !== l);
	}

	function limpiar() {
		carrito = [];
		inputCodigo?.focus();
	}

	function abrirCobro() {
		if (carrito.length === 0) return;
		errorVenta = '';
		ticket = null;
		ticketData = null;
		referencia = '';
		pagaConTxt = '';
		idCliente = null; // Público en General por defecto
		idMetodo = metodos[0]?.idMetodoPago ?? null;
		cobrando = true;
	}

	async function confirmar() {
		if (!$session || idMetodo == null) return;
		if (metodoSel?.requiereReferencia && !referencia.trim()) {
			errorVenta = 'Este método requiere una referencia (folio/voucher).';
			return;
		}
		if (esFiado && idCliente == null) {
			errorVenta = 'Selecciona el cliente para el fiado.';
			return;
		}
		if (!esFiado && pagaCon != null && pagaCon < total) {
			errorVenta = 'El pago es menor al total.';
			return;
		}

		procesando = true;
		errorVenta = '';
		try {
			const res = await api.registrarVenta({
				idUsuario: $session.idUsuario,
				idMetodoPago: idMetodo,
				idCorte: corte?.idCorte ?? null,
				idCliente: idCliente, // null = Público en General
				referenciaPago: referencia.trim() || null,
				pagoCon: esFiado ? null : pagaCon,
				items: carrito.map((l) => ({ idProducto: l.producto.idProducto, cantidad: l.cantidad }))
			});

			// Arma el ticket (snapshot del carrito antes de vaciarlo)
			ticketData = {
				folio: res.folio,
				fecha: fechaHora(new Date().toISOString()),
				cajero: $session.nombre,
				cliente:
					idCliente != null
						? (clientes.find((c) => c.idCliente === idCliente)?.nombre ?? 'Cliente')
						: 'Público en General',
				items: carrito.map((l) => ({
					nombre: l.producto.producto,
					cantidad: l.cantidad,
					precio: l.producto.precioUnitario
				})),
				total: res.total,
				metodo: metodoSel?.metodoPago ?? '',
				pagoCon: esFiado ? null : pagaCon,
				cambio: res.cambio
			};

			ticket = res;
			carrito = [];
			// refresca existencias en segundo plano
			api.listarProductos().then((p) => (productos = p));
		} catch (err) {
			errorVenta = String(err);
		} finally {
			procesando = false;
		}
	}

	function nuevaVenta() {
		cobrando = false;
		ticket = null;
		ticketData = null;
		inputCodigo?.focus();
	}

	function imprimirTicket() {
		window.print();
	}

	function descargarTicket() {
		if (ticketData) {
			generarTicketPDF(ticketData, negocio);
			toast('Ticket descargado (PDF)');
		}
	}
</script>

<div class="flex min-h-screen flex-col bg-slate-100 lg:h-screen lg:min-h-0">
	<!-- Header -->
	<header class="flex items-center justify-between border-b border-slate-200 bg-white px-5 py-3">
		<div class="flex items-center gap-3">
			<button onclick={() => goto(resolve('/'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
			<h1 class="text-lg font-semibold text-slate-800">Vender</h1>
		</div>
		<div class="flex items-center gap-4">
			<span class="text-sm tabular-nums text-slate-500">🕒 {reloj}</span>
			<span class="text-sm text-slate-400">{$session?.nombre}</span>
		</div>
	</header>

	{#if cargando}
		<p class="flex flex-1 items-center justify-center text-slate-400">Cargando…</p>
	{:else if !corte}
		<!-- No se puede vender sin caja abierta -->
		<div class="flex flex-1 items-start justify-center overflow-y-auto p-6">
			<div class="w-full max-w-sm rounded-2xl border border-slate-200 bg-white p-6 text-center shadow-sm">
				<div class="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-full bg-amber-100 text-2xl">💵</div>
				<h2 class="text-lg font-semibold text-slate-800">Abre la caja para vender</h2>
				<p class="mt-1 mb-4 text-sm text-slate-500">Debes abrir el turno con el efectivo inicial antes de registrar ventas.</p>
				<p class="mb-2 text-left text-sm font-medium text-slate-700">Cuenta el efectivo con el que abres</p>
				<div class="mb-3 text-left"><ContadorMonedas onTotal={(n) => (montoInicialCaja = n)} /></div>
				{#if errorCaja}<p class="mb-3 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{errorCaja}</p>{/if}
				<button onclick={abrirCaja} disabled={abriendoCaja} class="w-full rounded-lg bg-indigo-600 py-2.5 font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
					{abriendoCaja ? 'Abriendo…' : 'Abrir caja'}
				</button>
			</div>
		</div>
	{:else}
	<div class="flex flex-1 flex-col lg:min-h-0 lg:flex-row">
		<!-- Catálogo -->
		<section class="flex min-w-0 flex-1 flex-col p-4">
			<div class="mb-3 flex flex-wrap gap-2">
				<input
					bind:this={inputCodigo}
					bind:value={codigo}
					onkeydown={porCodigo}
					placeholder="📷 Escanea o teclea código + Enter"
					class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:w-72"
				/>
				<input
					bind:value={busqueda}
					placeholder="Buscar producto…"
					class="flex-1 rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500"
				/>
			</div>

			{#if aviso}
				<p class="mb-2 rounded-lg bg-amber-50 px-3 py-1.5 text-sm text-amber-700">{aviso}</p>
			{/if}

			<div class="min-h-0 flex-1 overflow-y-auto">
				{#if cargando}
					<p class="p-6 text-center text-slate-400">Cargando productos…</p>
				{:else if productos.length === 0}
					<p class="p-6 text-center text-slate-400">
						No hay productos. Agrégalos en Inventario para poder vender.
					</p>
				{:else}
					<div class="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4">
						{#each productosFiltrados as p (p.idProducto)}
							<button
								onclick={() => agregar(p)}
								class="flex flex-col rounded-xl border border-slate-200 bg-white p-3 text-left shadow-sm transition hover:border-indigo-300 hover:shadow"
							>
								<span class="line-clamp-2 text-sm font-medium text-slate-800">{p.producto}</span>
								<span class="mt-1 text-base font-semibold text-indigo-600">{pesos(p.precioUnitario)}</span>
								{#if p.tipo === 'Servicio'}
									<span class="text-xs text-sky-600">Servicio</span>
								{:else}
									<span class="text-xs text-slate-400">Existencia: {p.existencia}</span>
								{/if}
							</button>
						{/each}
					</div>
				{/if}
			</div>
		</section>

		<!-- Carrito -->
		<aside class="flex w-full flex-col border-t border-slate-200 bg-white lg:w-96 lg:border-l lg:border-t-0">
			<div class="flex items-center justify-between border-b border-slate-100 px-4 py-3">
				<h2 class="font-semibold text-slate-800">Ticket</h2>
				{#if carrito.length > 0}
					<button onclick={limpiar} class="text-xs text-slate-400 hover:text-red-500">Vaciar</button>
				{/if}
			</div>

			<div class="min-h-0 flex-1 overflow-y-auto">
				{#if carrito.length === 0}
					<p class="p-8 text-center text-sm text-slate-400">Agrega productos al ticket.</p>
				{:else}
					<ul class="divide-y divide-slate-100">
						{#each carrito as l (l.producto.idProducto)}
							<li class="flex items-center gap-2 px-4 py-2.5">
								<div class="min-w-0 flex-1">
									<p class="truncate text-sm font-medium text-slate-800">{l.producto.producto}</p>
									<p class="text-xs text-slate-400">{pesos(l.producto.precioUnitario)} c/u</p>
								</div>
								<div class="flex items-center gap-1">
									<button onclick={() => cambiarCantidad(l, -1)} class="h-6 w-6 rounded bg-slate-100 text-slate-600 hover:bg-slate-200">−</button>
									<span class="w-8 text-center text-sm font-medium">{l.cantidad}</span>
									<button onclick={() => cambiarCantidad(l, 1)} class="h-6 w-6 rounded bg-slate-100 text-slate-600 hover:bg-slate-200">+</button>
								</div>
								<span class="w-20 text-right text-sm font-semibold text-slate-800">
									{pesos(l.producto.precioUnitario * l.cantidad)}
								</span>
							</li>
						{/each}
					</ul>
				{/if}
			</div>

			<div class="border-t border-slate-200 p-4">
				<div class="mb-3 flex items-end justify-between">
					<span class="text-sm text-slate-500">{totalArticulos} artículo{totalArticulos === 1 ? '' : 's'}</span>
					<span class="text-2xl font-bold text-slate-800">{pesos(total)}</span>
				</div>
				<button
					onclick={abrirCobro}
					disabled={carrito.length === 0}
					class="w-full rounded-lg bg-indigo-600 py-3 font-semibold text-white transition hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-50"
				>
					Cobrar
				</button>
			</div>
		</aside>
	</div>
	{/if}
</div>

<!-- Modal de cobro -->
{#if cobrando}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="max-h-[90vh] w-full max-w-md overflow-y-auto rounded-2xl bg-white p-6 shadow-xl">
			{#if ticket && ticketData}
				<!-- Comprobante -->
				<div class="text-center">
					<div class="mx-auto mb-2 flex h-12 w-12 items-center justify-center rounded-full bg-green-100 text-2xl">✓</div>
					<h2 class="text-lg font-semibold text-slate-800">Venta registrada</h2>
					{#if ticket.cambio != null}
						<p class="mt-1 text-lg text-green-600">Cambio: {pesos(ticket.cambio)}</p>
					{/if}
				</div>

				<!-- Ticket: vista previa y lo que se imprime (ancho térmico) -->
				<div id="ticket-imprimible" class="mx-auto mt-3 w-[280px] border border-slate-200 bg-white p-3 text-left font-mono text-[11px] leading-tight text-slate-800">
					<p class="text-center text-sm font-bold">{negocio.NombreTienda || 'Mi Negocio'}</p>
					{#if negocio.Direccion}<p class="text-center text-[10px]">{negocio.Direccion}</p>{/if}
					{#if negocio.Telefono}<p class="text-center text-[10px]">Tel. {negocio.Telefono}</p>{/if}
					<div class="my-1 border-t border-dashed border-slate-300"></div>
					<div class="flex justify-between"><span>Folio: {ticketData.folio}</span><span>{ticketData.fecha}</span></div>
					<p>Cajero: {ticketData.cajero}</p>
					<p>Cliente: {ticketData.cliente}</p>
					<div class="my-1 border-t border-dashed border-slate-300"></div>
					{#each ticketData.items as it, i (i)}
						<div class="flex justify-between gap-2">
							<span class="truncate">{it.cantidad} x {it.nombre}</span>
							<span>{pesos(it.precio * it.cantidad)}</span>
						</div>
					{/each}
					<div class="my-1 border-t border-dashed border-slate-300"></div>
					<div class="flex justify-between font-bold"><span>TOTAL</span><span>{pesos(ticketData.total)}</span></div>
					<p>{ticketData.metodo}</p>
					{#if ticketData.pagoCon != null}
						<div class="flex justify-between"><span>Pagó: {pesos(ticketData.pagoCon)}</span><span>Cambio: {pesos(ticketData.cambio ?? 0)}</span></div>
					{/if}
					{#if negocio.PieTicket}<p class="mt-1 text-center text-[10px]">{negocio.PieTicket}</p>{/if}
					<p class="text-center">¡Gracias por su compra!</p>
				</div>

				<div class="mt-4 flex gap-2">
					<button onclick={imprimirTicket} class="flex-1 rounded-lg border border-slate-300 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50">🖨️ Imprimir</button>
					<button onclick={descargarTicket} class="flex-1 rounded-lg border border-slate-300 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50">📄 PDF</button>
				</div>
				<button onclick={nuevaVenta} class="mt-2 w-full rounded-lg bg-indigo-600 py-2.5 font-semibold text-white hover:bg-indigo-700">
					Nueva venta
				</button>
			{:else}
				<!-- Pago -->
				<div class="mb-4 flex items-center justify-between">
					<h2 class="text-lg font-semibold text-slate-800">Cobrar</h2>
					<button onclick={() => (cobrando = false)} class="text-slate-400 hover:text-slate-600" aria-label="Cerrar">✕</button>
				</div>

				<p class="mb-4 text-center text-3xl font-bold text-slate-800">{pesos(total)}</p>

				<label for="cli" class="mb-1 block text-sm font-medium text-slate-700">Cliente</label>
				<select id="cli" bind:value={idCliente} class="mb-4 w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500">
					<option value={null}>Público en General</option>
					{#each clientes as c (c.idCliente)}
						<option value={c.idCliente}>{c.nombre}{c.saldoFiado > 0 ? ` (debe ${pesos(c.saldoFiado)})` : ''}</option>
					{/each}
				</select>

				<p class="mb-1 text-sm font-medium text-slate-700">Método de pago</p>
				<div class="mb-4 grid grid-cols-2 gap-2">
					{#each metodos as m (m.idMetodoPago)}
						<button
							onclick={() => (idMetodo = m.idMetodoPago)}
							class="rounded-lg border px-3 py-2 text-sm font-medium transition {idMetodo === m.idMetodoPago
								? 'border-indigo-500 bg-indigo-50 text-indigo-700'
								: 'border-slate-200 text-slate-600 hover:bg-slate-50'}"
						>
							{m.metodoPago}
						</button>
					{/each}
				</div>

				{#if esFiado}
					{#if idCliente == null}
						<p class="mb-3 rounded-lg bg-amber-50 px-3 py-2 text-sm text-amber-700">
							Para vender a fiado, selecciona un cliente (no "Público en General").
						</p>
					{/if}
				{:else if metodoSel?.requiereReferencia}
					<label for="ref" class="mb-1 block text-sm font-medium text-slate-700">Referencia (folio/voucher)</label>
					<input id="ref" bind:value={referencia} class="mb-4 w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
				{:else}
					<label for="paga" class="mb-1 block text-sm font-medium text-slate-700">Paga con</label>
					<input id="paga" type="text" inputmode="decimal" bind:value={pagaConTxt} placeholder={total.toFixed(2)} class="mb-2 w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					{#if cambio != null}
						<p class="mb-3 text-right text-sm {cambio < 0 ? 'text-red-600' : 'text-green-600'}">
							Cambio: {pesos(cambio)}
						</p>
					{/if}
				{/if}

				{#if errorVenta}
					<p class="mb-3 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{errorVenta}</p>
				{/if}

				<button
					onclick={confirmar}
					disabled={procesando || idMetodo == null}
					class="w-full rounded-lg bg-green-600 py-3 font-semibold text-white transition hover:bg-green-700 disabled:cursor-not-allowed disabled:opacity-50"
				>
					{procesando ? 'Procesando…' : 'Confirmar venta'}
				</button>
			{/if}
		</div>
	</div>
{/if}

<style>
	/* Al imprimir, mostrar SOLO el ticket, a tamaño de impresora térmica. */
	@media print {
		:global(body *) {
			visibility: hidden !important;
		}
		:global(#ticket-imprimible),
		:global(#ticket-imprimible *) {
			visibility: visible !important;
		}
		:global(#ticket-imprimible) {
			position: absolute;
			left: 0;
			top: 0;
			width: 80mm;
			border: none !important;
		}
	}
	@page {
		size: 80mm auto;
		margin: 0;
	}
</style>
