<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, type Producto, type Categoria, type UnidadMedida } from '$lib/api';
	import { session } from '$lib/stores/session';
	import { modoVenta, giro, cargarAjustes } from '$lib/stores/ajustes';
	import { pesos } from '$lib/format';

	// Título/etiquetas según lo que vende el negocio.
	const titulo = $derived(
		$modoVenta === 'servicios' ? 'Servicios' : $modoVenta === 'ambos' ? 'Artículos' : 'Inventario'
	);

	let productos = $state<Producto[]>([]);
	let categorias = $state<Categoria[]>([]);
	let unidades = $state<UnidadMedida[]>([]);
	let cargando = $state(true);
	let busqueda = $state('');
	let verInactivos = $state(false);

	const filtrados = $derived(
		busqueda.trim()
			? productos.filter((p) =>
					(p.producto + ' ' + (p.codigoBarras ?? '')).toLowerCase().includes(busqueda.toLowerCase())
				)
			: productos
	);

	// --- Modal producto (crear/editar) ---
	let modalProd = $state(false);
	let editandoId = $state<number | null>(null);
	let nombre = $state('');
	let codigoBarras = $state('');
	let precioUnitario = $state<number | null>(0);
	let precioCosto = $state<number | null>(0);
	let tipo = $state<'Producto' | 'Servicio'>('Producto');
	let manejaCaducidad = $state(false);
	let loteInicial = $state('');
	let caducidadInicial = $state('');
	let stockMinimo = $state<number | null>(0);
	let existenciaInicial = $state<number | null>(0);
	let seVendePeso = $state(false);
	const esServicio = $derived(tipo === 'Servicio');
	const catSel = $derived(categorias.find((c) => c.idCategoria === idCat) ?? null);
	let activo = $state(true);
	let idCat = $state<number | null>(null);
	let idUni = $state<number | null>(null);
	let guardando = $state(false);
	let errorP = $state('');

	// --- Modal categoría ---
	let modalCat = $state(false);
	let nombreCat = $state('');
	let errorCat = $state('');

	// --- Modal ajuste de existencia ---
	let modalAjuste = $state(false);
	let ajusteProd = $state<Producto | null>(null);
	let tipoAjuste = $state<'Entrada' | 'Merma' | 'Ajuste'>('Entrada');
	let cantidadAjuste = $state<number | null>(null);
	let motivoAjuste = $state('');
	let ajustando = $state(false);
	let errorAjuste = $state('');

	onMount(() => {
		if ($session?.rol !== 'Administrador') {
			goto(resolve('/'));
			return;
		}
		cargarAjustes();
		cargar();
	});

	async function cargar() {
		cargando = true;
		try {
			const [pr, ca, un] = await Promise.all([
				api.listarProductos(verInactivos),
				api.listarCategorias(),
				api.listarUnidades()
			]);
			productos = pr;
			categorias = ca;
			unidades = un;
		} finally {
			cargando = false;
		}
	}

	function abrirNuevo() {
		editandoId = null;
		nombre = '';
		codigoBarras = '';
		precioUnitario = 0;
		precioCosto = 0;
		tipo = $modoVenta === 'servicios' ? 'Servicio' : 'Producto';
		manejaCaducidad = /farmac/i.test($giro); // por defecto on en farmacias
		loteInicial = '';
		caducidadInicial = '';
		stockMinimo = 0;
		existenciaInicial = 0;
		seVendePeso = false;
		activo = true;
		idCat = categorias[0]?.idCategoria ?? null;
		idUni = unidades[0]?.idUnidadMedida ?? null;
		errorP = '';
		modalProd = true;
	}

	function abrirEditar(p: Producto) {
		editandoId = p.idProducto;
		nombre = p.producto;
		codigoBarras = p.codigoBarras ?? '';
		precioUnitario = p.precioUnitario;
		precioCosto = p.precioCosto;
		tipo = p.tipo === 'Servicio' ? 'Servicio' : 'Producto';
		manejaCaducidad = p.manejaCaducidad;
		loteInicial = '';
		caducidadInicial = '';
		stockMinimo = p.stockMinimo;
		existenciaInicial = 0;
		seVendePeso = p.seVendePeso;
		activo = p.activo;
		idCat = p.idCategoria;
		idUni = p.idUnidadMedida;
		errorP = '';
		modalProd = true;
	}

	async function guardarProducto() {
		if (!nombre.trim()) {
			errorP = 'El nombre es obligatorio.';
			return;
		}
		guardando = true;
		errorP = '';
		try {
			if (editandoId != null) {
				await api.actualizarProducto(editandoId, {
					producto: nombre.trim(),
					codigoBarras: codigoBarras.trim() || null,
					precioUnitario: precioUnitario ?? 0,
					precioCosto: precioCosto ?? 0,
					tipo,
					manejaCaducidad: esServicio ? false : manejaCaducidad,
					seVendePeso: esServicio ? false : seVendePeso,
					stockMinimo: esServicio ? 0 : (stockMinimo ?? 0),
					idUnidadMedida: idUni,
					idCategoria: idCat,
					activo
				});
			} else {
				const lotear = !esServicio && manejaCaducidad;
				await api.crearProducto({
					producto: nombre.trim(),
					codigoBarras: codigoBarras.trim() || null,
					precioUnitario: precioUnitario ?? 0,
					precioCosto: precioCosto ?? 0,
					tipo,
					manejaCaducidad: esServicio ? false : manejaCaducidad,
					seVendePeso: esServicio ? false : seVendePeso,
					stockMinimo: esServicio ? 0 : (stockMinimo ?? 0),
					idUnidadMedida: idUni,
					idCategoria: idCat,
					existenciaInicial: esServicio ? 0 : (existenciaInicial ?? 0),
					loteInicial: lotear ? loteInicial.trim() || null : null,
					caducidadInicial: lotear ? caducidadInicial || null : null
				});
			}
			await cargar();
			modalProd = false;
		} catch (e) {
			errorP = String(e);
		} finally {
			guardando = false;
		}
	}

	async function guardarCategoria() {
		if (!nombreCat.trim()) {
			errorCat = 'Escribe un nombre.';
			return;
		}
		errorCat = '';
		try {
			const id = await api.crearCategoria(nombreCat.trim());
			categorias = await api.listarCategorias();
			idCat = id;
			nombreCat = '';
			modalCat = false;
		} catch (e) {
			errorCat = String(e);
		}
	}

	function abrirAjuste(p: Producto) {
		ajusteProd = p;
		tipoAjuste = 'Entrada';
		cantidadAjuste = null;
		motivoAjuste = '';
		errorAjuste = '';
		modalAjuste = true;
	}

	async function guardarAjuste() {
		if (!ajusteProd || cantidadAjuste == null || cantidadAjuste < 0) {
			errorAjuste = 'Escribe una cantidad válida.';
			return;
		}
		ajustando = true;
		errorAjuste = '';
		try {
			await api.ajustarInventario(
				ajusteProd.idProducto,
				tipoAjuste,
				cantidadAjuste,
				motivoAjuste.trim() || null,
				$session?.idUsuario ?? null
			);
			await cargar();
			modalAjuste = false;
		} catch (e) {
			errorAjuste = String(e);
		} finally {
			ajustando = false;
		}
	}

	function bajoStock(p: Producto): boolean {
		return p.stockMinimo > 0 && p.existencia <= p.stockMinimo;
	}
</script>

<div class="min-h-screen bg-slate-100">
	<header class="flex items-center justify-between border-b border-slate-200 bg-white px-5 py-3">
		<div class="flex items-center gap-3">
			<button onclick={() => goto(resolve('/'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
			<h1 class="text-lg font-semibold text-slate-800">{titulo}</h1>
		</div>
		<div class="flex gap-2">
			<button onclick={() => goto(resolve('/categorias'))} class="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium text-slate-600 hover:bg-slate-50">
				Categorías
			</button>
			<button onclick={abrirNuevo} class="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700">
				+ Nuevo {$modoVenta === 'servicios' ? 'servicio' : 'artículo'}
			</button>
		</div>
	</header>

	<main class="mx-auto max-w-7xl p-6">
		<div class="mb-4 flex items-center gap-4">
			<input
				bind:value={busqueda}
				placeholder="Buscar producto o código…"
				class="w-full max-w-sm rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500"
			/>
			<label class="flex shrink-0 items-center gap-2 text-sm text-slate-600">
				<input type="checkbox" bind:checked={verInactivos} onchange={cargar} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
				Ver inactivos
			</label>
		</div>

		{#if cargando}
			<p class="py-10 text-center text-slate-400">Cargando…</p>
		{:else if productos.length === 0}
			<div class="rounded-xl border border-dashed border-slate-300 bg-white py-12 text-center">
				<p class="text-slate-500">Aún no hay {$modoVenta === 'servicios' ? 'servicios' : 'artículos'}.</p>
				<button onclick={abrirNuevo} class="mt-3 rounded-lg bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700">
					Agregar el primero
				</button>
			</div>
		{:else}
			<div class="overflow-x-auto rounded-xl border border-slate-200 bg-white">
				<table class="w-full text-sm">
					<thead class="bg-slate-50 text-left text-xs uppercase tracking-wide text-slate-500">
						<tr>
							<th class="px-4 py-2.5 font-medium">Producto</th>
							<th class="px-4 py-2.5 font-medium">Código</th>
							<th class="px-4 py-2.5 font-medium">Categoría</th>
							<th class="px-4 py-2.5 text-right font-medium">Precio</th>
							<th class="px-4 py-2.5 text-right font-medium">Existencia</th>
							<th class="px-4 py-2.5 text-right font-medium">Acciones</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-slate-100">
						{#each filtrados as p (p.idProducto)}
							<tr class="hover:bg-slate-50 {p.activo ? '' : 'opacity-50'}">
								<td class="px-4 py-2.5 font-medium text-slate-800">
									{p.producto}
									{#if p.tipo === 'Servicio'}<span class="ml-1 rounded bg-sky-100 px-1.5 py-0.5 text-[10px] font-medium text-sky-700">servicio</span>{/if}
									{#if p.manejaCaducidad}<span class="ml-1 rounded bg-purple-100 px-1.5 py-0.5 text-[10px] font-medium text-purple-700">lotes</span>{/if}
									{#if !p.activo}<span class="ml-1 rounded bg-slate-200 px-1.5 py-0.5 text-[10px] font-medium text-slate-500">inactivo</span>{/if}
								</td>
								<td class="px-4 py-2.5 font-mono text-xs text-slate-500">{p.codigoBarras ?? '—'}</td>
								<td class="px-4 py-2.5 text-slate-600">{p.categoria ?? '—'}</td>
								<td class="px-4 py-2.5 text-right text-slate-800">{pesos(p.precioUnitario)}</td>
								<td class="px-4 py-2.5 text-right">
									{#if p.tipo === 'Servicio'}
										<span class="text-slate-300">—</span>
									{:else}
										<span class="font-medium {bajoStock(p) ? 'text-red-600' : 'text-slate-700'}">{p.existencia}</span>
										{#if bajoStock(p)}<span class="ml-1 rounded bg-red-100 px-1.5 py-0.5 text-[10px] font-medium text-red-600">bajo</span>{/if}
									{/if}
								</td>
								<td class="px-4 py-2.5 text-right whitespace-nowrap">
									{#if p.tipo !== 'Servicio'}
										<button onclick={() => abrirAjuste(p)} class="rounded-lg border border-slate-200 px-2.5 py-1 text-xs font-medium text-slate-600 hover:bg-slate-50">Ajustar</button>
									{/if}
									<button onclick={() => abrirEditar(p)} class="rounded-lg border border-indigo-200 bg-indigo-50 px-2.5 py-1 text-xs font-medium text-indigo-700 hover:bg-indigo-100">Editar</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</main>
</div>

<!-- Modal crear/editar producto -->
{#if modalProd}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="max-h-[90vh] w-full max-w-lg overflow-y-auto rounded-2xl bg-white p-6 shadow-xl">
			<div class="mb-4 flex items-center justify-between">
				<h2 class="text-lg font-semibold text-slate-800">{editandoId != null ? 'Editar' : 'Nuevo'} {esServicio ? 'servicio' : 'artículo'}</h2>
				<button onclick={() => (modalProd = false)} class="text-slate-400 hover:text-slate-600" aria-label="Cerrar">✕</button>
			</div>

			<div class="space-y-3">
				{#if $modoVenta === 'ambos'}
					<div>
						<span class="mb-1 block text-sm font-medium text-slate-700">Tipo</span>
						<div class="grid grid-cols-2 gap-2">
							<button type="button" onclick={() => (tipo = 'Producto')} class="rounded-lg border px-3 py-2 text-sm font-medium transition {!esServicio ? 'border-indigo-500 bg-indigo-50 text-indigo-700' : 'border-slate-300 text-slate-600 hover:bg-slate-50'}">📦 Producto</button>
							<button type="button" onclick={() => (tipo = 'Servicio')} class="rounded-lg border px-3 py-2 text-sm font-medium transition {esServicio ? 'border-indigo-500 bg-indigo-50 text-indigo-700' : 'border-slate-300 text-slate-600 hover:bg-slate-50'}">🛠️ Servicio</button>
						</div>
						<p class="mt-1 text-xs text-slate-400">Los servicios no manejan inventario ni stock.</p>
					</div>
				{/if}
				<div>
					<label for="nom" class="mb-1 block text-sm font-medium text-slate-700">Nombre</label>
					<input id="nom" bind:value={nombre} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
				</div>
				<div>
					<label for="cb" class="mb-1 block text-sm font-medium text-slate-700">Código de barras (opcional)</label>
					<input id="cb" bind:value={codigoBarras} class="w-full rounded-lg border-slate-300 font-mono shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
				</div>

				<div class="grid grid-cols-2 gap-3">
					<div>
						<label for="pv" class="mb-1 block text-sm font-medium text-slate-700">Precio de venta</label>
						<input id="pv" type="number" step="0.01" min="0" bind:value={precioUnitario} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					</div>
					<div>
						<label for="pc" class="mb-1 block text-sm font-medium text-slate-700">Precio de costo</label>
						<input id="pc" type="number" step="0.01" min="0" bind:value={precioCosto} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					</div>
				</div>

				<div class="grid grid-cols-2 gap-3">
					<div>
						<div class="mb-1 flex items-center justify-between">
							<label for="cat" class="text-sm font-medium text-slate-700">Categoría</label>
							<button type="button" onclick={() => { errorCat = ''; modalCat = true; }} class="text-xs font-medium text-indigo-600 hover:underline">+ nueva</button>
						</div>
						<select id="cat" bind:value={idCat} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500">
							{#each categorias as c (c.idCategoria)}
								<option value={c.idCategoria} title={c.descripcion ?? ''}>{c.categoria}</option>
							{/each}
						</select>
					</div>
					<div>
						<label for="uni" class="mb-1 block text-sm font-medium text-slate-700">Unidad</label>
						<select id="uni" bind:value={idUni} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500">
							{#each unidades as u (u.idUnidadMedida)}
								<option value={u.idUnidadMedida}>{u.unidadMedida}</option>
							{/each}
						</select>
					</div>
				</div>

				{#if catSel?.descripcion}
					<p class="-mt-1 rounded-lg bg-slate-50 px-3 py-1.5 text-xs text-slate-500">
						<span class="font-medium text-slate-600">{catSel.categoria}:</span> {catSel.descripcion}
					</p>
				{/if}

				{#if !esServicio}
					<label class="flex items-center gap-2 text-sm text-slate-700">
						<input type="checkbox" bind:checked={manejaCaducidad} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
						🗓️ Maneja caducidad y lote (farmacia)
					</label>

					<div class="grid grid-cols-2 gap-3">
						<div>
							<label for="si" class="mb-1 block text-sm font-medium text-slate-700">Stock mínimo</label>
							<input id="si" type="number" step="0.01" min="0" bind:value={stockMinimo} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
						</div>
						{#if editandoId == null}
							<div>
								<label for="ei" class="mb-1 block text-sm font-medium text-slate-700">Existencia inicial</label>
								<input id="ei" type="number" step="0.01" min="0" bind:value={existenciaInicial} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
							</div>
						{/if}
					</div>

					{#if manejaCaducidad && editandoId == null}
						<div class="grid grid-cols-2 gap-3 rounded-lg bg-slate-50 p-3">
							<div>
								<label for="li" class="mb-1 block text-sm font-medium text-slate-700">Lote inicial (opcional)</label>
								<input id="li" bind:value={loteInicial} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
							</div>
							<div>
								<label for="ci" class="mb-1 block text-sm font-medium text-slate-700">Caducidad</label>
								<input id="ci" type="date" bind:value={caducidadInicial} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
							</div>
						</div>
					{:else if manejaCaducidad}
						<p class="rounded-lg bg-slate-50 px-3 py-2 text-xs text-slate-500">
							El stock se lleva por <strong>lotes</strong>. Agrega lotes con su caducidad desde <strong>Compras</strong> o con el botón <strong>Ajustar</strong>.
						</p>
					{/if}

					{#if !manejaCaducidad}
						<label class="flex items-center gap-2 text-sm text-slate-700">
							<input type="checkbox" bind:checked={seVendePeso} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
							Se vende por peso (kg/litro)
						</label>
					{/if}
				{/if}

				{#if editandoId != null}
					<label class="flex items-center gap-2 text-sm text-slate-700">
						<input type="checkbox" bind:checked={activo} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
						Activo (visible para vender)
					</label>
				{/if}

				{#if errorP}
					<p class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{errorP}</p>
				{/if}

				<button onclick={guardarProducto} disabled={guardando} class="w-full rounded-lg bg-indigo-600 py-2.5 font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
					{guardando ? 'Guardando…' : editandoId != null ? 'Guardar cambios' : 'Guardar producto'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Modal nueva categoría -->
{#if modalCat}
	<div class="fixed inset-0 z-20 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="w-full max-w-xs rounded-2xl bg-white p-6 shadow-xl">
			<h2 class="mb-3 text-base font-semibold text-slate-800">Nueva categoría</h2>
			<input bind:value={nombreCat} placeholder="Ej. Bebidas" class="mb-2 w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
			{#if errorCat}
				<p class="mb-2 text-sm text-red-700">{errorCat}</p>
			{/if}
			<div class="flex gap-2">
				<button onclick={() => (modalCat = false)} class="flex-1 rounded-lg border border-slate-300 py-2 text-sm text-slate-600 hover:bg-slate-50">Cancelar</button>
				<button onclick={guardarCategoria} class="flex-1 rounded-lg bg-indigo-600 py-2 text-sm font-semibold text-white hover:bg-indigo-700">Crear</button>
			</div>
		</div>
	</div>
{/if}

<!-- Modal ajuste de existencia -->
{#if modalAjuste && ajusteProd}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="w-full max-w-xs rounded-2xl bg-white p-6 shadow-xl">
			<h2 class="mb-1 text-base font-semibold text-slate-800">Ajustar existencia</h2>
			<p class="mb-3 text-sm text-slate-500">{ajusteProd.producto} · actual: <strong>{ajusteProd.existencia}</strong></p>

			<div class="mb-3 grid grid-cols-3 gap-1.5">
				{#each ['Entrada', 'Merma', 'Ajuste'] as t (t)}
					<button
						onclick={() => (tipoAjuste = t as 'Entrada' | 'Merma' | 'Ajuste')}
						class="rounded-lg border px-2 py-1.5 text-xs font-medium {tipoAjuste === t ? 'border-indigo-500 bg-indigo-50 text-indigo-700' : 'border-slate-200 text-slate-600 hover:bg-slate-50'}"
					>
						{t}
					</button>
				{/each}
			</div>

			<label for="ca" class="mb-1 block text-sm font-medium text-slate-700">
				{tipoAjuste === 'Ajuste' ? 'Existencia real (conteo)' : 'Cantidad'}
			</label>
			<input id="ca" type="number" step="0.01" min="0" bind:value={cantidadAjuste} class="mb-2 w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
			<input bind:value={motivoAjuste} placeholder="Motivo (opcional)" class="mb-3 w-full rounded-lg border-slate-300 text-sm shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />

			{#if errorAjuste}
				<p class="mb-2 text-sm text-red-700">{errorAjuste}</p>
			{/if}
			<div class="flex gap-2">
				<button onclick={() => (modalAjuste = false)} class="flex-1 rounded-lg border border-slate-300 py-2 text-sm text-slate-600 hover:bg-slate-50">Cancelar</button>
				<button onclick={guardarAjuste} disabled={ajustando} class="flex-1 rounded-lg bg-indigo-600 py-2 text-sm font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
					{ajustando ? 'Guardando…' : 'Aplicar'}
				</button>
			</div>
		</div>
	</div>
{/if}
