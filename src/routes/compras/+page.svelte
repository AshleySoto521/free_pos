<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, type Producto, type Proveedor } from '$lib/api';
	import { session } from '$lib/stores/session';
	import { pesos } from '$lib/format';

	type Linea = {
		producto: Producto;
		cantidad: number;
		costoUnitario: number;
		lote: string;
		caducidad: string;
	};

	let productos = $state<Producto[]>([]);
	let proveedores = $state<Proveedor[]>([]);
	let cargando = $state(true);

	let idProveedor = $state<number | null>(null);
	let folio = $state('');
	let actualizarCosto = $state(true);
	let lineas = $state<Linea[]>([]);
	let seleccion = $state<number | null>(null);

	let guardando = $state(false);
	let error = $state('');
	let exito = $state('');

	const total = $derived(lineas.reduce((s, l) => s + l.cantidad * l.costoUnitario, 0));

	onMount(async () => {
		if ($session?.rol !== 'Administrador') {
			goto(resolve('/'));
			return;
		}
		try {
			const [pr, pv] = await Promise.all([api.listarProductos(), api.listarProveedores()]);
			productos = pr;
			proveedores = pv;
		} finally {
			cargando = false;
		}
	});

	function agregar() {
		if (seleccion == null) return;
		const p = productos.find((x) => x.idProducto === seleccion);
		if (!p) return;
		if (!lineas.some((l) => l.producto.idProducto === p.idProducto)) {
			lineas.push({ producto: p, cantidad: 1, costoUnitario: p.precioCosto, lote: '', caducidad: '' });
		}
		seleccion = null;
		exito = '';
	}

	function quitar(l: Linea) {
		lineas = lineas.filter((x) => x !== l);
	}

	async function guardar() {
		if (lineas.length === 0) {
			error = 'Agrega al menos un producto.';
			return;
		}
		guardando = true;
		error = '';
		try {
			const res = await api.registrarCompra({
				idProveedor,
				idUsuario: $session?.idUsuario ?? null,
				folio: folio.trim() || null,
				actualizarCosto,
				items: lineas.map((l) => ({
					idProducto: l.producto.idProducto,
					cantidad: l.cantidad,
					costoUnitario: l.costoUnitario,
					lote: l.producto.manejaCaducidad ? l.lote.trim() || null : null,
					caducidad: l.producto.manejaCaducidad ? l.caducidad || null : null
				}))
			});
			exito = `Compra registrada por ${pesos(res.total)}. Inventario actualizado.`;
			lineas = [];
			folio = '';
			productos = await api.listarProductos();
		} catch (e) {
			error = String(e);
		} finally {
			guardando = false;
		}
	}
</script>

<div class="min-h-screen bg-slate-100">
	<header class="flex items-center gap-3 border-b border-slate-200 bg-white px-5 py-3">
		<button onclick={() => goto(resolve('/'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
		<h1 class="text-lg font-semibold text-slate-800">Compras / Reabastecer</h1>
	</header>

	<main class="mx-auto max-w-3xl p-6">
		{#if cargando}
			<p class="py-10 text-center text-slate-400">Cargando…</p>
		{:else if productos.length === 0}
			<p class="rounded-xl border border-dashed border-slate-300 bg-white py-12 text-center text-slate-500">
				Primero crea productos en Inventario.
			</p>
		{:else}
			<div class="space-y-4 rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
				<div class="grid grid-cols-2 gap-3">
					<div>
						<label for="prov" class="mb-1 block text-sm font-medium text-slate-700">Proveedor (opcional)</label>
						<select id="prov" bind:value={idProveedor} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500">
							<option value={null}>—</option>
							{#each proveedores as p (p.idProveedor)}
								<option value={p.idProveedor}>{p.proveedor}</option>
							{/each}
						</select>
					</div>
					<div>
						<label for="fol" class="mb-1 block text-sm font-medium text-slate-700">Folio / factura (opcional)</label>
						<input id="fol" bind:value={folio} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					</div>
				</div>

				<div>
					<label for="add" class="mb-1 block text-sm font-medium text-slate-700">Agregar producto</label>
					<div class="flex gap-2">
						<select id="add" bind:value={seleccion} class="flex-1 rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500">
							<option value={null}>Selecciona…</option>
							{#each productos as p (p.idProducto)}
								<option value={p.idProducto}>{p.producto}</option>
							{/each}
						</select>
						<button onclick={agregar} class="rounded-lg bg-slate-800 px-4 text-sm font-semibold text-white hover:bg-slate-900">Agregar</button>
					</div>
				</div>

				{#if lineas.length > 0}
					<div class="overflow-hidden rounded-lg border border-slate-200">
						<table class="w-full text-sm">
							<thead class="bg-slate-50 text-left text-xs uppercase tracking-wide text-slate-500">
								<tr>
									<th class="px-3 py-2 font-medium">Producto</th>
									<th class="px-3 py-2 font-medium">Cantidad</th>
									<th class="px-3 py-2 font-medium">Costo unit.</th>
									<th class="px-3 py-2 text-right font-medium">Subtotal</th>
									<th class="px-3 py-2"></th>
								</tr>
							</thead>
							<tbody class="divide-y divide-slate-100">
								{#each lineas as l (l.producto.idProducto)}
									<tr>
										<td class="px-3 py-2 font-medium text-slate-800">
											{l.producto.producto}
											{#if l.producto.manejaCaducidad}
												<div class="mt-1 flex flex-wrap gap-1">
													<input bind:value={l.lote} placeholder="Lote" class="w-24 rounded border-slate-300 py-1 text-xs shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
													<input type="date" bind:value={l.caducidad} class="rounded border-slate-300 py-1 text-xs shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
												</div>
											{/if}
										</td>
										<td class="px-3 py-2">
											<input type="number" step="0.01" min="0" bind:value={l.cantidad} class="w-20 rounded-lg border-slate-300 py-1 text-sm shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
										</td>
										<td class="px-3 py-2">
											<input type="number" step="0.01" min="0" bind:value={l.costoUnitario} class="w-24 rounded-lg border-slate-300 py-1 text-sm shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
										</td>
										<td class="px-3 py-2 text-right font-medium text-slate-700">{pesos(l.cantidad * l.costoUnitario)}</td>
										<td class="px-3 py-2 text-right">
											<button onclick={() => quitar(l)} class="text-slate-400 hover:text-red-500" aria-label="Quitar">✕</button>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}

				<label class="flex items-center gap-2 text-sm text-slate-700">
					<input type="checkbox" bind:checked={actualizarCosto} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
					Actualizar el precio de costo de los productos con esta compra
				</label>

				<div class="flex items-center justify-between border-t border-slate-100 pt-3">
					<span class="text-lg font-bold text-slate-800">Total: {pesos(total)}</span>
					<button onclick={guardar} disabled={guardando || lineas.length === 0} class="rounded-lg bg-indigo-600 px-6 py-2.5 font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
						{guardando ? 'Guardando…' : 'Registrar compra'}
					</button>
				</div>

				{#if error}<p class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>{/if}
				{#if exito}<p class="rounded-lg bg-green-50 px-3 py-2 text-sm text-green-700">{exito}</p>{/if}
			</div>
		{/if}
	</main>
</div>
