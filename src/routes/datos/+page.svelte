<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import {
		api,
		type FilaProductoImport,
		type FilaClienteImport,
		type FilaCategoriaImport,
		type FilaProveedorImport,
		type ResultadoImport
	} from '$lib/api';
	import { session } from '$lib/stores/session';
	import { toast } from '$lib/stores/toast';
	import { exportarXLSX, parsearArchivo, encabezadosDe } from '$lib/xlsx';

	let exportando = $state('');

	onMount(() => {
		if ($session?.rol !== 'Administrador') goto(resolve('/'));
	});

	// ---------------- EXPORTAR ----------------
	async function exportarProductos() {
		exportando = 'productos';
		try {
			const p = await api.listarProductos(true);
			exportarXLSX(
				'productos',
				['PRODUCTO', 'CODIGOBARRAS', 'PRECIOVENTA', 'PRECIOCOSTO', 'EXISTENCIA', 'CATEGORIA'],
				p.map((x) => [x.producto, x.codigoBarras ?? '', x.precioUnitario, x.precioCosto, x.existencia, x.categoria ?? ''])
			);
			toast('Descarga realizada');
		} catch (e) {
			toast(String(e), 'error');
		} finally {
			exportando = '';
		}
	}
	async function exportarClientes() {
		exportando = 'clientes';
		try {
			const c = await api.listarClientes(true);
			exportarXLSX('clientes', ['NOMBRE', 'TELEFONO', 'SALDOFIADO'], c.map((x) => [x.nombre, x.telefono ?? '', x.saldoFiado]));
			toast('Descarga realizada');
		} catch (e) {
			toast(String(e), 'error');
		} finally {
			exportando = '';
		}
	}
	async function exportarProveedores() {
		exportando = 'proveedores';
		try {
			const p = await api.listarProveedores(true);
			exportarXLSX('proveedores', ['PROVEEDOR', 'CONTACTO', 'TELEFONO'], p.map((x) => [x.proveedor, x.contacto ?? '', x.telefono ?? '']));
			toast('Descarga realizada');
		} catch (e) {
			toast(String(e), 'error');
		} finally {
			exportando = '';
		}
	}
	async function exportarCategorias() {
		exportando = 'categorias';
		try {
			const c = await api.listarCategorias(true);
			exportarXLSX('categorias', ['CATEGORIA', 'DESCRIPCION'], c.map((x) => [x.categoria, x.descripcion ?? '']));
			toast('Descarga realizada');
		} catch (e) {
			toast(String(e), 'error');
		} finally {
			exportando = '';
		}
	}

	// ---------------- IMPORTAR PRODUCTOS ----------------
	let prodFilas = $state<FilaProductoImport[]>([]);
	let prodErrores = $state<string[]>([]);
	let prodArchivo = $state('');
	let prodResultado = $state<ResultadoImport | null>(null);
	let prodImportando = $state(false);

	async function onArchivoProductos(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file) return;
		prodArchivo = file.name;
		prodResultado = null;
		prodFilas = [];
		prodErrores = [];
		const filas = await parsearArchivo(file);
		const heads = encabezadosDe(filas);
		const faltan = ['PRODUCTO', 'PRECIOVENTA'].filter((h) => !heads.includes(h));
		if (faltan.length) {
			prodErrores = [`Faltan columnas obligatorias: ${faltan.join(', ')}`];
			return;
		}
		const out: FilaProductoImport[] = [];
		filas.forEach((r, i) => {
			if (!r.PRODUCTO) return;
			const precio = Number(r.PRECIOVENTA);
			if (Number.isNaN(precio)) {
				prodErrores.push(`Fila ${i + 2}: PRECIOVENTA no es un número`);
				return;
			}
			out.push({
				producto: r.PRODUCTO,
				codigoBarras: r.CODIGOBARRAS || null,
				precioVenta: precio,
				precioCosto: r.PRECIOCOSTO ? Number(r.PRECIOCOSTO) : null,
				existencia: r.EXISTENCIA ? Number(r.EXISTENCIA) : null,
				categoria: r.CATEGORIA || null
			});
		});
		prodFilas = out;
	}

	async function importarProductos() {
		prodImportando = true;
		try {
			prodResultado = await api.importarProductos(prodFilas);
			prodFilas = [];
			toast(`Importados: ${prodResultado.insertados}`);
		} catch (e) {
			prodErrores = [String(e)];
			toast('Error al importar', 'error');
		} finally {
			prodImportando = false;
		}
	}

	// ---------------- IMPORTAR CLIENTES ----------------
	let cliFilas = $state<FilaClienteImport[]>([]);
	let cliErrores = $state<string[]>([]);
	let cliArchivo = $state('');
	let cliResultado = $state<ResultadoImport | null>(null);
	let cliImportando = $state(false);

	async function onArchivoClientes(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file) return;
		cliArchivo = file.name;
		cliResultado = null;
		cliFilas = [];
		cliErrores = [];
		const filas = await parsearArchivo(file);
		const heads = encabezadosDe(filas);
		if (!heads.includes('NOMBRE')) {
			cliErrores = ['Falta la columna obligatoria: NOMBRE'];
			return;
		}
		cliFilas = filas
			.filter((r) => r.NOMBRE)
			.map((r) => ({ nombre: r.NOMBRE, telefono: r.TELEFONO || null }));
	}

	async function importarClientes() {
		cliImportando = true;
		try {
			cliResultado = await api.importarClientes(cliFilas);
			cliFilas = [];
			toast(`Importados: ${cliResultado.insertados}`);
		} catch (e) {
			cliErrores = [String(e)];
			toast('Error al importar', 'error');
		} finally {
			cliImportando = false;
		}
	}

	// ---------------- IMPORTAR CATEGORÍAS ----------------
	let catFilas = $state<FilaCategoriaImport[]>([]);
	let catErrores = $state<string[]>([]);
	let catArchivo = $state('');
	let catResultado = $state<ResultadoImport | null>(null);
	let catImportando = $state(false);

	async function onArchivoCategorias(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file) return;
		catArchivo = file.name;
		catResultado = null;
		catFilas = [];
		catErrores = [];
		const filas = await parsearArchivo(file);
		if (!encabezadosDe(filas).includes('CATEGORIA')) {
			catErrores = ['Falta la columna obligatoria: CATEGORIA'];
			return;
		}
		catFilas = filas
			.filter((r) => r.CATEGORIA)
			.map((r) => ({ categoria: r.CATEGORIA, descripcion: r.DESCRIPCION || null }));
	}

	async function importarCategorias() {
		catImportando = true;
		try {
			catResultado = await api.importarCategorias(catFilas);
			catFilas = [];
			toast(`Importadas: ${catResultado.insertados}`);
		} catch (e) {
			catErrores = [String(e)];
			toast('Error al importar', 'error');
		} finally {
			catImportando = false;
		}
	}

	// ---------------- IMPORTAR PROVEEDORES ----------------
	let provFilas = $state<FilaProveedorImport[]>([]);
	let provErrores = $state<string[]>([]);
	let provArchivo = $state('');
	let provResultado = $state<ResultadoImport | null>(null);
	let provImportando = $state(false);

	async function onArchivoProveedores(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file) return;
		provArchivo = file.name;
		provResultado = null;
		provFilas = [];
		provErrores = [];
		const filas = await parsearArchivo(file);
		if (!encabezadosDe(filas).includes('PROVEEDOR')) {
			provErrores = ['Falta la columna obligatoria: PROVEEDOR'];
			return;
		}
		provFilas = filas
			.filter((r) => r.PROVEEDOR)
			.map((r) => ({ proveedor: r.PROVEEDOR, contacto: r.CONTACTO || null, telefono: r.TELEFONO || null }));
	}

	async function importarProveedores() {
		provImportando = true;
		try {
			provResultado = await api.importarProveedores(provFilas);
			provFilas = [];
			toast(`Importados: ${provResultado.insertados}`);
		} catch (e) {
			provErrores = [String(e)];
			toast('Error al importar', 'error');
		} finally {
			provImportando = false;
		}
	}
</script>

<div class="min-h-screen bg-slate-100">
	<header class="flex items-center gap-3 border-b border-slate-200 bg-white px-5 py-3">
		<button onclick={() => goto(resolve('/catalogos'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
		<h1 class="text-lg font-semibold text-slate-800">Importar / Exportar</h1>
	</header>

	<main class="mx-auto max-w-2xl space-y-6 p-6">
		<!-- EXPORTAR -->
		<section class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
			<h2 class="mb-1 font-semibold text-slate-800">Exportar a Excel</h2>
			<p class="mb-3 text-sm text-slate-500">Descarga el catálogo completo en .xlsx.</p>
			<div class="flex flex-wrap gap-2">
				<button onclick={exportarProductos} disabled={exportando === 'productos'} class="rounded-lg border border-slate-300 px-3 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-50">📦 Productos</button>
				<button onclick={exportarClientes} disabled={exportando === 'clientes'} class="rounded-lg border border-slate-300 px-3 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-50">🧾 Clientes</button>
				<button onclick={exportarProveedores} disabled={exportando === 'proveedores'} class="rounded-lg border border-slate-300 px-3 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-50">🏷️ Proveedores</button>
				<button onclick={exportarCategorias} disabled={exportando === 'categorias'} class="rounded-lg border border-slate-300 px-3 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-50">⚖️ Categorías</button>
			</div>
		</section>

		<!-- IMPORTAR PRODUCTOS -->
		<section class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
			<h2 class="mb-1 font-semibold text-slate-800">Importar productos</h2>
			<p class="mb-3 text-sm text-slate-500">
				Archivo .xlsx o .csv con columnas: <code class="rounded bg-slate-100 px-1 text-xs">PRODUCTO</code>,
				<code class="rounded bg-slate-100 px-1 text-xs">PRECIOVENTA</code> (obligatorias) y opcionalmente
				CODIGOBARRAS, PRECIOCOSTO, EXISTENCIA, CATEGORIA. Los textos se guardan en MAYÚSCULAS.
			</p>
			<input type="file" accept=".xlsx,.csv" onchange={onArchivoProductos} class="mb-2 block w-full text-sm text-slate-600 file:mr-3 file:rounded-lg file:border-0 file:bg-indigo-50 file:px-3 file:py-2 file:text-sm file:font-medium file:text-indigo-700 hover:file:bg-indigo-100" />

			{#each prodErrores as err, i (i)}
				<p class="mt-1 text-sm text-red-700">{err}</p>
			{/each}

			{#if prodFilas.length > 0}
				<div class="mt-3 flex items-center justify-between rounded-lg bg-slate-50 px-3 py-2 text-sm">
					<span class="text-slate-600">{prodArchivo}: <strong>{prodFilas.length}</strong> productos listos</span>
					<button onclick={importarProductos} disabled={prodImportando} class="rounded-lg bg-indigo-600 px-4 py-1.5 text-sm font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
						{prodImportando ? 'Importando…' : 'Importar'}
					</button>
				</div>
			{/if}

			{#if prodResultado}
				<div class="mt-3 rounded-lg bg-green-50 px-3 py-2 text-sm text-green-800">
					✓ Importados: {prodResultado.insertados} · Omitidos: {prodResultado.omitidos}
					{#if prodResultado.errores.length}
						<ul class="mt-1 list-inside list-disc text-red-700">
							{#each prodResultado.errores.slice(0, 10) as e, i (i)}<li>{e}</li>{/each}
						</ul>
					{/if}
				</div>
			{/if}
		</section>

		<!-- IMPORTAR CLIENTES -->
		<section class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
			<h2 class="mb-1 font-semibold text-slate-800">Importar clientes</h2>
			<p class="mb-3 text-sm text-slate-500">
				Archivo .xlsx o .csv con columna <code class="rounded bg-slate-100 px-1 text-xs">NOMBRE</code>
				(obligatoria) y opcionalmente TELEFONO.
			</p>
			<input type="file" accept=".xlsx,.csv" onchange={onArchivoClientes} class="mb-2 block w-full text-sm text-slate-600 file:mr-3 file:rounded-lg file:border-0 file:bg-indigo-50 file:px-3 file:py-2 file:text-sm file:font-medium file:text-indigo-700 hover:file:bg-indigo-100" />

			{#each cliErrores as err, i (i)}
				<p class="mt-1 text-sm text-red-700">{err}</p>
			{/each}

			{#if cliFilas.length > 0}
				<div class="mt-3 flex items-center justify-between rounded-lg bg-slate-50 px-3 py-2 text-sm">
					<span class="text-slate-600">{cliArchivo}: <strong>{cliFilas.length}</strong> clientes listos</span>
					<button onclick={importarClientes} disabled={cliImportando} class="rounded-lg bg-indigo-600 px-4 py-1.5 text-sm font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
						{cliImportando ? 'Importando…' : 'Importar'}
					</button>
				</div>
			{/if}

			{#if cliResultado}
				<div class="mt-3 rounded-lg bg-green-50 px-3 py-2 text-sm text-green-800">
					✓ Importados: {cliResultado.insertados} · Omitidos: {cliResultado.omitidos}
				</div>
			{/if}
		</section>

		<!-- IMPORTAR CATEGORÍAS -->
		<section class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
			<h2 class="mb-1 font-semibold text-slate-800">Importar categorías</h2>
			<p class="mb-3 text-sm text-slate-500">
				Archivo .xlsx o .csv con columna <code class="rounded bg-slate-100 px-1 text-xs">CATEGORIA</code>
				(obligatoria) y opcionalmente DESCRIPCION. Se omiten las que ya existen.
			</p>
			<input type="file" accept=".xlsx,.csv" onchange={onArchivoCategorias} class="mb-2 block w-full text-sm text-slate-600 file:mr-3 file:rounded-lg file:border-0 file:bg-indigo-50 file:px-3 file:py-2 file:text-sm file:font-medium file:text-indigo-700 hover:file:bg-indigo-100" />

			{#each catErrores as err, i (i)}
				<p class="mt-1 text-sm text-red-700">{err}</p>
			{/each}

			{#if catFilas.length > 0}
				<div class="mt-3 flex items-center justify-between rounded-lg bg-slate-50 px-3 py-2 text-sm">
					<span class="text-slate-600">{catArchivo}: <strong>{catFilas.length}</strong> categorías listas</span>
					<button onclick={importarCategorias} disabled={catImportando} class="rounded-lg bg-indigo-600 px-4 py-1.5 text-sm font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
						{catImportando ? 'Importando…' : 'Importar'}
					</button>
				</div>
			{/if}

			{#if catResultado}
				<div class="mt-3 rounded-lg bg-green-50 px-3 py-2 text-sm text-green-800">
					✓ Importadas: {catResultado.insertados} · Omitidas: {catResultado.omitidos}
				</div>
			{/if}
		</section>

		<!-- IMPORTAR PROVEEDORES -->
		<section class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
			<h2 class="mb-1 font-semibold text-slate-800">Importar proveedores</h2>
			<p class="mb-3 text-sm text-slate-500">
				Archivo .xlsx o .csv con columna <code class="rounded bg-slate-100 px-1 text-xs">PROVEEDOR</code>
				(obligatoria) y opcionalmente CONTACTO, TELEFONO. Se omiten los que ya existen.
			</p>
			<input type="file" accept=".xlsx,.csv" onchange={onArchivoProveedores} class="mb-2 block w-full text-sm text-slate-600 file:mr-3 file:rounded-lg file:border-0 file:bg-indigo-50 file:px-3 file:py-2 file:text-sm file:font-medium file:text-indigo-700 hover:file:bg-indigo-100" />

			{#each provErrores as err, i (i)}
				<p class="mt-1 text-sm text-red-700">{err}</p>
			{/each}

			{#if provFilas.length > 0}
				<div class="mt-3 flex items-center justify-between rounded-lg bg-slate-50 px-3 py-2 text-sm">
					<span class="text-slate-600">{provArchivo}: <strong>{provFilas.length}</strong> proveedores listos</span>
					<button onclick={importarProveedores} disabled={provImportando} class="rounded-lg bg-indigo-600 px-4 py-1.5 text-sm font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
						{provImportando ? 'Importando…' : 'Importar'}
					</button>
				</div>
			{/if}

			{#if provResultado}
				<div class="mt-3 rounded-lg bg-green-50 px-3 py-2 text-sm text-green-800">
					✓ Importados: {provResultado.insertados} · Omitidos: {provResultado.omitidos}
				</div>
			{/if}
		</section>
	</main>
</div>
