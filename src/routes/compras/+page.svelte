<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, type Producto, type Proveedor } from '$lib/api';
	import { session } from '$lib/stores/session';
	import { pesos } from '$lib/format';
	import { parsearArchivo, encabezadosDe, descargarPlantillaCSV } from '$lib/xlsx';

	type Linea = {
		producto: Producto;
		cantidad: number;
		costoUnitario: number;
		precioVenta: number | null; // opcional: si viene, actualiza el precio de venta
		lote: string;
		caducidad: string;
	};

	let productos = $state<Producto[]>([]);
	let proveedores = $state<Proveedor[]>([]);
	let cargando = $state(true);

	let idProveedor = $state<number | null>(null);
	let folio = $state('');
	let lineas = $state<Linea[]>([]);
	let busquedaProd = $state('');
	let marcados = $state<Record<number, boolean>>({});
	let importError = $state('');
	let faltantes = $state<string[]>([]); // productos del archivo que no existen

	let guardando = $state(false);
	let error = $state('');
	let exito = $state('');

	const total = $derived(lineas.reduce((s, l) => s + l.cantidad * l.costoUnitario, 0));
	const prodFiltrados = $derived(
		productos.filter(
			(p) =>
				p.tipo !== 'Servicio' &&
				(busquedaProd.trim() === '' ||
					(p.producto + ' ' + (p.codigoBarras ?? ''))
						.toLowerCase()
						.includes(busquedaProd.toLowerCase()))
		)
	);
	const numMarcados = $derived(Object.values(marcados).filter(Boolean).length);

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

	function agregarMarcados() {
		for (const p of productos) {
			if (
				marcados[p.idProducto] &&
				p.tipo !== 'Servicio' &&
				!lineas.some((l) => l.producto.idProducto === p.idProducto)
			) {
				lineas.push({ producto: p, cantidad: 1, costoUnitario: p.precioCosto, precioVenta: null, lote: '', caducidad: '' });
			}
		}
		marcados = {};
		busquedaProd = '';
		exito = '';
	}

	async function onArchivoCompra(e: Event) {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		importError = '';
		const filas = await parsearArchivo(file);
		const heads = encabezadosDe(filas);
		if (!heads.includes('CANTIDAD') || (!heads.includes('PRODUCTO') && !heads.includes('CODIGOBARRAS'))) {
			importError = 'El archivo necesita columnas CANTIDAD y PRODUCTO (o CODIGOBARRAS).';
			input.value = '';
			return;
		}
		const errores: string[] = [];
		const noExisten: string[] = [];
		// Acumula lo del archivo (suma duplicados DENTRO del mismo archivo).
		const delArchivo = new Map<
			number,
			{ p: Producto; cant: number; costo: number; pv: number | null; lote: string; caducidad: string }
		>();
		filas.forEach((r, i) => {
			const cant = Number(r.CANTIDAD);
			if (!r.CANTIDAD || Number.isNaN(cant) || cant <= 0) return;
			let p: Producto | undefined;
			if (r.CODIGOBARRAS)
				p = productos.find((x) => (x.codigoBarras ?? '').toUpperCase() === r.CODIGOBARRAS.toUpperCase());
			if (!p && r.PRODUCTO)
				p = productos.find((x) => x.producto.toUpperCase() === r.PRODUCTO.toUpperCase());
			if (!p) {
				noExisten.push(r.PRODUCTO || r.CODIGOBARRAS || '?');
				return;
			}
			if (p.tipo === 'Servicio') {
				errores.push(`Fila ${i + 2}: "${p.producto}" es servicio`);
				return;
			}
			// Costo de compra (acepta varios nombres de columna).
			const costoRaw = r.PRECIOCOMPRA ?? r.PRECIOUNITARIO ?? r.PRECIOCOSTO ?? r.COSTO;
			const costoNum =
				costoRaw != null && String(costoRaw).trim() !== '' ? Number(costoRaw) : p.precioCosto;
			const costo = Number.isNaN(costoNum) ? p.precioCosto : costoNum;
			// Precio de venta OPCIONAL: si viene, actualiza el del producto.
			const pvNum = Number(r.PRECIOVENTA);
			const pv =
				r.PRECIOVENTA != null && String(r.PRECIOVENTA).trim() !== '' && !Number.isNaN(pvNum) && pvNum > 0
					? pvNum
					: null;
			const prev = delArchivo.get(p.idProducto);
			if (prev) {
				prev.cant += cant;
				if (pv != null) prev.pv = pv;
			} else
				delArchivo.set(p.idProducto, { p, cant, costo, pv, lote: r.LOTE || '', caducidad: r.CADUCIDAD || '' });
		});
		// REEMPLAZA la cantidad de los productos del archivo (idempotente: re-cargar
		// la misma factura NO duplica). No toca líneas que agregaste a mano.
		for (const info of delArchivo.values()) {
			const existente = lineas.find((l) => l.producto.idProducto === info.p.idProducto);
			if (existente) {
				existente.cantidad = info.cant;
				existente.costoUnitario = info.costo;
				existente.precioVenta = info.pv;
				existente.lote = info.lote;
				existente.caducidad = info.caducidad;
			} else {
				lineas.push({
					producto: info.p,
					cantidad: info.cant,
					costoUnitario: info.costo,
					precioVenta: info.pv,
					lote: info.lote,
					caducidad: info.caducidad
				});
			}
		}
		faltantes = noExisten;
		importError = errores.length ? `Ignorados: ${errores.join(' · ')}` : '';
		input.value = '';
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
				items: lineas.map((l) => ({
					idProducto: l.producto.idProducto,
					cantidad: l.cantidad,
					costoUnitario: l.costoUnitario,
					precioVenta: l.precioVenta,
					lote: l.producto.manejaCaducidad ? l.lote.trim() || null : null,
					caducidad: l.producto.manejaCaducidad ? l.caducidad || null : null
				}))
			});
			exito = `Compra registrada por ${pesos(res.total)}. Inventario actualizado.`;
			lineas = [];
			folio = '';
			faltantes = [];
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
					<div class="mb-1 flex items-center justify-between">
						<span class="text-sm font-medium text-slate-700">Agregar productos</span>
						<div class="flex items-center gap-3">
							<button type="button" onclick={() => descargarPlantillaCSV('compras', ['PRODUCTO', 'CODIGOBARRAS', 'PRECIOCOMPRA', 'CANTIDAD', 'PRECIOVENTA', 'LOTE', 'CADUCIDAD'], ['COCA COLA 600ML', '7501055300013', '12.50', '24', '18', '', ''])} class="text-xs font-medium text-indigo-600 hover:underline">⬇️ Plantilla</button>
							<label class="cursor-pointer text-xs font-medium text-indigo-600 hover:underline" title="Columnas: CANTIDAD + PRODUCTO (o CODIGOBARRAS) + PRECIOCOMPRA (costo). Opcional: PRECIOVENTA (actualiza el precio de venta), LOTE, CADUCIDAD.">
								📥 Importar Excel/CSV
								<input type="file" accept=".xlsx,.csv" onchange={onArchivoCompra} class="hidden" />
							</label>
						</div>
					</div>
					<input bind:value={busquedaProd} placeholder="Buscar producto…" class="mb-2 w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					<div class="max-h-44 overflow-y-auto rounded-lg border border-slate-200">
						{#each prodFiltrados as p (p.idProducto)}
							<label class="flex cursor-pointer items-center gap-2 border-b border-slate-100 px-3 py-1.5 text-sm last:border-0 hover:bg-slate-50">
								<input type="checkbox" bind:checked={marcados[p.idProducto]} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
								<span class="flex-1 text-slate-700">{p.producto}</span>
								<span class="text-xs text-slate-400">Exist: {p.existencia}</span>
							</label>
						{:else}
							<p class="px-3 py-2 text-sm text-slate-400">Sin coincidencias.</p>
						{/each}
					</div>
					<button onclick={agregarMarcados} disabled={numMarcados === 0} class="mt-2 w-full rounded-lg bg-slate-800 py-2 text-sm font-semibold text-white hover:bg-slate-900 disabled:opacity-50">
						Agregar seleccionados ({numMarcados})
					</button>
					<p class="mt-1 text-xs text-slate-400">¿No aparece un producto? Si es nuevo, agrégalo en <strong>Inventario</strong> primero (con su precio y unidad).</p>
					{#if importError}<p class="mt-1 text-xs text-red-600">{importError}</p>{/if}
				</div>

				{#if lineas.length > 0}
					<p class="text-xs text-slate-400">💡 Esta lista es la <strong>compra</strong>: se <strong>suma</strong> a tu inventario al registrar. No es tu existencia actual.</p>
				{/if}
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

				{#if faltantes.length > 0}
					<div class="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-800">
						<p>⚠️ La factura trae <strong>{faltantes.length}</strong> producto(s) que <strong>no existen</strong>: {faltantes.join(', ')}.</p>
						<p class="mt-1 text-xs">Agrégalos en <strong>Inventario</strong> y vuelve a cargar la factura <strong>completa</strong>. No se puede registrar hasta que existan todos (así no se parte la factura ni se duplican productos).</p>
					</div>
				{/if}

				<div class="flex items-center justify-between border-t border-slate-100 pt-3">
					<span class="text-lg font-bold text-slate-800">Total: {pesos(total)}</span>
					<button onclick={guardar} disabled={guardando || lineas.length === 0 || faltantes.length > 0} class="rounded-lg bg-indigo-600 px-6 py-2.5 font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
						{guardando ? 'Guardando…' : 'Registrar compra'}
					</button>
				</div>

				{#if error}<p class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>{/if}
				{#if exito}<p class="rounded-lg bg-green-50 px-3 py-2 text-sm text-green-700">{exito}</p>{/if}
			</div>
		{/if}
	</main>
</div>
