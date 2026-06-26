<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import {
		api,
		type VentaResumen,
		type ReporteVentas,
		type DetalleVentaLinea
	} from '$lib/api';
	import { session } from '$lib/stores/session';
	import { pesos, horaLocal } from '$lib/format';

	const esAdmin = $derived($session?.rol === 'Administrador');

	function hoyStr() {
		const d = new Date();
		return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
	}

	let desde = $state(hoyStr());
	let hasta = $state(hoyStr());
	let ventas = $state<VentaResumen[]>([]);
	let reporte = $state<ReporteVentas | null>(null);
	let cargando = $state(true);

	let expandedId = $state<number | null>(null);
	let detalle = $state<DetalleVentaLinea[]>([]);

	let ventaACancelar = $state<VentaResumen | null>(null);
	let disposicion = $state<'regresar' | 'merma' | 'descartar'>('regresar');
	let cancelando = $state(false);
	let errorCancel = $state('');

	const opcionesDisp: { valor: 'regresar' | 'merma' | 'descartar'; label: string; desc: string }[] = [
		{ valor: 'regresar', label: '↩️ Regresar al inventario', desc: 'La mercancía vuelve al stock (en buen estado).' },
		{ valor: 'merma', label: '🗑️ Merma', desc: 'No vuelve al stock; se registra como pérdida.' },
		{ valor: 'descartar', label: '🚫 No regresar', desc: 'No vuelve al stock ni se registra como merma.' }
	];

	onMount(() => {
		if (!esAdmin) {
			goto(resolve('/'));
			return;
		}
		cargar();
	});

	async function cargar() {
		cargando = true;
		expandedId = null;
		try {
			const [v, r] = await Promise.all([
				api.listarVentas(desde, hasta),
				api.reporteVentas(desde, hasta)
			]);
			ventas = v;
			reporte = r;
		} finally {
			cargando = false;
		}
	}

	async function toggleDetalle(v: VentaResumen) {
		if (expandedId === v.idVenta) {
			expandedId = null;
			return;
		}
		detalle = await api.detalleVenta(v.idVenta);
		expandedId = v.idVenta;
	}

	async function confirmarCancelacion() {
		if (!ventaACancelar || !$session) return;
		cancelando = true;
		errorCancel = '';
		try {
			await api.cancelarVenta(ventaACancelar.idVenta, $session.idUsuario, disposicion);
			ventaACancelar = null;
			await cargar();
		} catch (e) {
			errorCancel = String(e);
		} finally {
			cancelando = false;
		}
	}

	const hora = (f: string) => horaLocal(f);
</script>

<div class="min-h-screen bg-slate-100">
	<header class="flex items-center gap-3 border-b border-slate-200 bg-white px-5 py-3">
		<button onclick={() => goto(resolve('/'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
		<h1 class="text-lg font-semibold text-slate-800">Ventas</h1>
	</header>

	<main class="mx-auto max-w-6xl p-6">
		<!-- Filtro de fechas -->
		<div class="mb-4 flex flex-wrap items-end gap-3">
			<div>
				<label for="d" class="mb-1 block text-xs font-medium text-slate-500">Desde</label>
				<input id="d" type="date" bind:value={desde} class="rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
			</div>
			<div>
				<label for="h" class="mb-1 block text-xs font-medium text-slate-500">Hasta</label>
				<input id="h" type="date" bind:value={hasta} class="rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
			</div>
			<button onclick={cargar} class="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700">Aplicar</button>
		</div>

		<!-- Cierre / resumen -->
		{#if reporte}
			<div class="mb-5 rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
				<div class="mb-3 flex items-end justify-between">
					<div>
						<p class="text-sm text-slate-500">Total vendido</p>
						<p class="text-3xl font-bold text-slate-800">{pesos(reporte.total)}</p>
					</div>
					<div class="text-right text-sm text-slate-500">
						<p>{reporte.tickets} ticket{reporte.tickets === 1 ? '' : 's'}</p>
						{#if reporte.canceladas > 0}<p class="text-red-500">{reporte.canceladas} cancelada{reporte.canceladas === 1 ? '' : 's'}</p>{/if}
					</div>
				</div>
				{#if reporte.metodos.length > 0}
					<div class="flex flex-wrap gap-2 border-t border-slate-100 pt-3">
						{#each reporte.metodos as m (m.metodoPago)}
							<span class="rounded-lg bg-slate-50 px-3 py-1.5 text-sm text-slate-600">
								{m.metodoPago}: <strong class="text-slate-800">{pesos(m.total)}</strong>
								<span class="text-xs text-slate-400">({m.tickets})</span>
							</span>
						{/each}
					</div>
				{/if}
			</div>
		{/if}

		<!-- Historial -->
		{#if cargando}
			<p class="py-10 text-center text-slate-400">Cargando…</p>
		{:else if ventas.length === 0}
			<p class="rounded-xl border border-dashed border-slate-300 bg-white py-12 text-center text-slate-500">
				No hay ventas en este rango.
			</p>
		{:else}
			<div class="overflow-x-auto rounded-xl border border-slate-200 bg-white">
				<table class="w-full text-sm">
					<thead class="bg-slate-50 text-left text-xs uppercase tracking-wide text-slate-500">
						<tr>
							<th class="px-4 py-2.5 font-medium">Folio</th>
							<th class="px-4 py-2.5 font-medium">Hora</th>
							<th class="px-4 py-2.5 font-medium">Método</th>
							<th class="px-4 py-2.5 font-medium">Cajero</th>
							<th class="px-4 py-2.5 text-right font-medium">Total</th>
							<th class="px-4 py-2.5 font-medium">Estado</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-slate-100">
						{#each ventas as v (v.idVenta)}
							<tr class="cursor-pointer hover:bg-slate-50 {v.estatus === 'Cancelada' ? 'opacity-50' : ''}" onclick={() => toggleDetalle(v)}>
								<td class="px-4 py-2.5 font-medium text-slate-800">{v.folio ?? v.idVenta}</td>
								<td class="px-4 py-2.5 text-slate-500">{hora(v.fechaVenta)}</td>
								<td class="px-4 py-2.5 text-slate-600">{v.metodoPago}{v.cliente ? ` · ${v.cliente}` : ''}</td>
								<td class="px-4 py-2.5 text-slate-600">{v.usuario}</td>
								<td class="px-4 py-2.5 text-right font-semibold text-slate-800">{pesos(v.total)}</td>
								<td class="px-4 py-2.5">
									<span class="rounded-full px-2 py-0.5 text-xs font-medium {v.estatus === 'Completada' ? 'bg-green-100 text-green-700' : 'bg-red-100 text-red-600'}">{v.estatus}</span>
								</td>
							</tr>
							{#if expandedId === v.idVenta}
								<tr class="bg-slate-50">
									<td colspan="6" class="px-4 py-3">
										<ul class="mb-2 space-y-1 text-sm">
											{#each detalle as l (l.producto)}
												<li class="flex justify-between text-slate-600">
													<span>{l.cantidad} × {l.producto}</span>
													<span>{pesos(l.precio * l.cantidad)}</span>
												</li>
											{/each}
										</ul>
										{#if v.estatus === 'Completada' && esAdmin}
											<button onclick={(e) => { e.stopPropagation(); ventaACancelar = v; disposicion = 'regresar'; errorCancel = ''; }} class="rounded-lg border border-red-200 bg-red-50 px-3 py-1.5 text-xs font-medium text-red-700 hover:bg-red-100">
												Cancelar venta
											</button>
										{:else if v.estatus === 'Completada'}
											<p class="text-xs text-slate-400">Solo un administrador puede cancelar ventas.</p>
										{/if}
									</td>
								</tr>
							{/if}
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</main>
</div>

<!-- Confirmar cancelación -->
{#if ventaACancelar}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="w-full max-w-sm rounded-2xl bg-white p-6 shadow-xl">
			<h2 class="mb-2 text-lg font-semibold text-slate-800">Cancelar venta</h2>
			<p class="mb-3 text-sm text-slate-600">
				Se cancelará el folio <strong>{ventaACancelar.folio ?? ventaACancelar.idVenta}</strong> por
				<strong>{pesos(ventaACancelar.total)}</strong>{#if ventaACancelar.cliente}, y se ajusta el fiado del cliente{/if}. Esto no se puede deshacer.
			</p>

			<span class="mb-1 block text-sm font-medium text-slate-700">¿Qué pasa con la mercancía?</span>
			<div class="mb-1 space-y-2">
				{#each opcionesDisp as o (o.valor)}
					<button
						type="button"
						onclick={() => (disposicion = o.valor)}
						class="block w-full rounded-lg border px-3 py-2 text-left transition {disposicion === o.valor
							? 'border-indigo-500 bg-indigo-50'
							: 'border-slate-300 hover:bg-slate-50'}"
					>
						<span class="block text-sm font-medium text-slate-800">{o.label}</span>
						<span class="block text-xs text-slate-500">{o.desc}</span>
					</button>
				{/each}
			</div>
			<p class="mb-3 text-xs text-slate-400">Solo aplica a productos; los servicios no manejan inventario.</p>

			{#if errorCancel}
				<p class="mb-2 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{errorCancel}</p>
			{/if}
			<div class="flex gap-2">
				<button onclick={() => (ventaACancelar = null)} class="flex-1 rounded-lg border border-slate-300 py-2 text-sm text-slate-600 hover:bg-slate-50">No</button>
				<button onclick={confirmarCancelacion} disabled={cancelando} class="flex-1 rounded-lg bg-red-600 py-2 text-sm font-semibold text-white hover:bg-red-700 disabled:opacity-50">
					{cancelando ? 'Cancelando…' : 'Sí, cancelar'}
				</button>
			</div>
		</div>
	</div>
{/if}
