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
	import { exportarXLSX, parsearArchivo, encabezadosDe, descargarPlantillaCSV } from '$lib/xlsx';

	let exportando = $state('');

	// ¿la celda trae un valor real? (no vacía, no solo espacios)
	const tieneValor = (v: unknown) =>
		v !== undefined && v !== null && String(v).trim() !== '';

	// "Se vende por peso": interpreta Sí/Si/1/X/Verdadero como verdadero; lo demás, falso.
	const SI_PESO = new Set(['SI', 'SÍ', 'S', '1', 'X', 'TRUE', 'VERDADERO', 'PESO', 'GRANEL', 'Y']);
	const esPeso = (v: unknown) => SI_PESO.has(String(v ?? '').trim().toUpperCase());

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
				['PRODUCTO', 'CODIGOBARRAS', 'PRECIOVENTA', 'PRECIOCOSTO', 'EXISTENCIA', 'CATEGORIA', 'UNIDAD', 'SEVENDEPESO'],
				p.map((x) => [x.producto, x.codigoBarras ?? '', x.precioUnitario, x.precioCosto, x.existencia, x.categoria ?? '', x.unidad ?? '', x.seVendePeso ? 'Sí' : 'No'])
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
			exportarXLSX('clientes', ['NOMBRE', 'TELEFONO', 'EMAIL', 'SALDOFIADO'], c.map((x) => [x.nombre, x.telefono ?? '', x.email ?? '', x.saldoFiado]));
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
			exportarXLSX('proveedores', ['PROVEEDOR', 'CONTACTO', 'TELEFONO', 'EMAIL'], p.map((x) => [x.proveedor, x.contacto ?? '', x.telefono ?? '', x.email ?? '']));
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
		const faltan = ['PRODUCTO', 'PRECIOVENTA', 'PRECIOCOSTO', 'CATEGORIA', 'UNIDAD'].filter(
			(h) => !heads.includes(h)
		);
		if (faltan.length) {
			prodErrores = [`Faltan columnas obligatorias: ${faltan.join(', ')}`];
			return;
		}
		const out: FilaProductoImport[] = [];
		const errs: string[] = [];
		filas.forEach((r, i) => {
			if (!tieneValor(r.PRODUCTO)) return; // renglón vacío → se ignora
			const renglon = i + 2;
			const faltantes: string[] = [];
			const precio = Number(r.PRECIOVENTA);
			if (!tieneValor(r.PRECIOVENTA) || Number.isNaN(precio)) faltantes.push('PRECIOVENTA');
			const costo = Number(r.PRECIOCOSTO);
			if (!tieneValor(r.PRECIOCOSTO) || Number.isNaN(costo)) faltantes.push('PRECIOCOSTO');
			if (!tieneValor(r.CATEGORIA)) faltantes.push('CATEGORIA');
			if (!tieneValor(r.UNIDAD)) faltantes.push('UNIDAD');
			if (faltantes.length) {
				errs.push(`Fila ${renglon} (${r.PRODUCTO}): falta ${faltantes.join(', ')}`);
				return;
			}
			out.push({
				producto: r.PRODUCTO,
				codigoBarras: r.CODIGOBARRAS || null,
				precioVenta: precio,
				precioCosto: costo,
				existencia: null, // catálogo: 0 existencia, el stock entra por Compras
				categoria: r.CATEGORIA,
				unidad: r.UNIDAD,
				seVendePeso: heads.includes('SEVENDEPESO') ? esPeso(r.SEVENDEPESO) : null
			});
		});
		prodErrores = errs;
		prodFilas = errs.length ? [] : out;
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

	// ---------------- INICIAR INVENTARIO (carga inicial) ----------------
	let iniFilas = $state<FilaProductoImport[]>([]);
	let iniErrores = $state<string[]>([]);
	let iniArchivo = $state('');
	let iniResultado = $state<ResultadoImport | null>(null);
	let iniCargando = $state(false);
	let iniConfirmo = $state(false);

	async function onArchivoIniciar(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file) return;
		iniArchivo = file.name;
		iniResultado = null;
		iniFilas = [];
		iniErrores = [];
		const filas = await parsearArchivo(file);
		const heads = encabezadosDe(filas);
		const faltan = [
			'PRODUCTO',
			'PRECIOVENTA',
			'PRECIOCOSTO',
			'EXISTENCIA',
			'CATEGORIA',
			'UNIDAD'
		].filter((h) => !heads.includes(h));
		if (faltan.length) {
			iniErrores = [`Faltan columnas obligatorias: ${faltan.join(', ')}`];
			return;
		}
		const out: FilaProductoImport[] = [];
		const errs: string[] = [];
		filas.forEach((r, i) => {
			if (!tieneValor(r.PRODUCTO)) return; // renglón vacío → se ignora
			const renglon = i + 2;
			const faltantes: string[] = [];
			const precio = Number(r.PRECIOVENTA);
			if (!tieneValor(r.PRECIOVENTA) || Number.isNaN(precio)) faltantes.push('PRECIOVENTA');
			const costo = Number(r.PRECIOCOSTO);
			if (!tieneValor(r.PRECIOCOSTO) || Number.isNaN(costo)) faltantes.push('PRECIOCOSTO');
			const exist = Number(r.EXISTENCIA);
			if (!tieneValor(r.EXISTENCIA) || Number.isNaN(exist)) faltantes.push('EXISTENCIA');
			if (!tieneValor(r.CATEGORIA)) faltantes.push('CATEGORIA');
			if (!tieneValor(r.UNIDAD)) faltantes.push('UNIDAD');
			if (faltantes.length) {
				errs.push(`Fila ${renglon} (${r.PRODUCTO}): falta ${faltantes.join(', ')}`);
				return;
			}
			out.push({
				producto: r.PRODUCTO,
				codigoBarras: r.CODIGOBARRAS || null,
				precioVenta: precio,
				precioCosto: costo,
				existencia: exist,
				categoria: r.CATEGORIA,
				unidad: r.UNIDAD,
				seVendePeso: heads.includes('SEVENDEPESO') ? esPeso(r.SEVENDEPESO) : null
			});
		});
		iniErrores = errs;
		iniFilas = errs.length ? [] : out;
	}

	async function iniciarInventario() {
		iniCargando = true;
		try {
			iniResultado = await api.iniciarInventario(iniFilas);
			iniFilas = [];
			iniConfirmo = false;
			toast(`Inventario inicial: ${iniResultado.insertados} productos`);
		} catch (e) {
			iniErrores = [String(e)];
			toast('Error al iniciar inventario', 'error');
		} finally {
			iniCargando = false;
		}
	}

	// ---------------- ACTUALIZAR PRECIOS (en masa) ----------------
	let preFilas = $state<FilaProductoImport[]>([]);
	let preErrores = $state<string[]>([]);
	let preArchivo = $state('');
	let preResultado = $state<ResultadoImport | null>(null);
	let preCargando = $state(false);

	async function onArchivoPrecios(e: Event) {
		const file = (e.target as HTMLInputElement).files?.[0];
		if (!file) return;
		preArchivo = file.name;
		preResultado = null;
		preFilas = [];
		preErrores = [];
		const filas = await parsearArchivo(file);
		const heads = encabezadosDe(filas);
		const faltan = ['PRODUCTO', 'PRECIOVENTA'].filter((h) => !heads.includes(h));
		if (faltan.length) {
			preErrores = [`Faltan columnas obligatorias: ${faltan.join(', ')}`];
			return;
		}
		const out: FilaProductoImport[] = [];
		const errs: string[] = [];
		filas.forEach((r, i) => {
			if (!tieneValor(r.PRODUCTO)) return;
			const renglon = i + 2;
			const precio = Number(r.PRECIOVENTA);
			if (!tieneValor(r.PRECIOVENTA) || Number.isNaN(precio)) {
				errs.push(`Fila ${renglon} (${r.PRODUCTO}): falta PRECIOVENTA`);
				return;
			}
			const costoOk = tieneValor(r.PRECIOCOSTO) && !Number.isNaN(Number(r.PRECIOCOSTO));
			out.push({
				producto: r.PRODUCTO,
				codigoBarras: r.CODIGOBARRAS || null,
				precioVenta: precio,
				precioCosto: costoOk ? Number(r.PRECIOCOSTO) : null,
				existencia: null,
				categoria: null,
				unidad: null
			});
		});
		preErrores = errs;
		preFilas = errs.length ? [] : out;
	}

	async function actualizarPrecios() {
		preCargando = true;
		try {
			preResultado = await api.actualizarPrecios(preFilas);
			preFilas = [];
			toast(`Precios actualizados: ${preResultado.insertados}`);
		} catch (e) {
			preErrores = [String(e)];
			toast('Error al actualizar precios', 'error');
		} finally {
			preCargando = false;
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
		const faltan = ['NOMBRE', 'TELEFONO'].filter((h) => !heads.includes(h));
		if (faltan.length) {
			cliErrores = [`Faltan columnas obligatorias: ${faltan.join(', ')}`];
			return;
		}
		const out: FilaClienteImport[] = [];
		const errs: string[] = [];
		filas.forEach((r, i) => {
			if (!tieneValor(r.NOMBRE)) return;
			if (!tieneValor(r.TELEFONO)) {
				errs.push(`Fila ${i + 2} (${r.NOMBRE}): falta TELEFONO`);
				return;
			}
			out.push({ nombre: r.NOMBRE, telefono: r.TELEFONO, email: r.EMAIL || null });
		});
		cliErrores = errs;
		cliFilas = errs.length ? [] : out;
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
		const heads = encabezadosDe(filas);
		const faltan = ['PROVEEDOR', 'TELEFONO'].filter((h) => !heads.includes(h));
		if (faltan.length) {
			provErrores = [`Faltan columnas obligatorias: ${faltan.join(', ')}`];
			return;
		}
		const out: FilaProveedorImport[] = [];
		const errs: string[] = [];
		filas.forEach((r, i) => {
			if (!tieneValor(r.PROVEEDOR)) return;
			if (!tieneValor(r.TELEFONO)) {
				errs.push(`Fila ${i + 2} (${r.PROVEEDOR}): falta TELEFONO`);
				return;
			}
			out.push({
				proveedor: r.PROVEEDOR,
				contacto: r.CONTACTO || null,
				telefono: r.TELEFONO,
				email: r.EMAIL || null
			});
		});
		provErrores = errs;
		provFilas = errs.length ? [] : out;
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
			<h2 class="mb-1 font-semibold text-slate-800">Agregar productos nuevos (catálogo)</h2>
			<p class="mb-2 text-sm text-slate-500">
				Para <strong>expansión</strong>: dar de alta varios productos nuevos de golpe. Se crean con
				<strong>0 existencia</strong> — el stock entra después por <strong>Compras</strong>. No toca tu
				PEPS ni choca con compras.
			</p>
			<p class="mb-3 text-sm text-slate-500">
				Archivo .xlsx o .csv. Columnas <strong>obligatorias</strong>:
				<code class="rounded bg-slate-100 px-1 text-xs">PRODUCTO</code>,
				<code class="rounded bg-slate-100 px-1 text-xs">PRECIOVENTA</code>,
				<code class="rounded bg-slate-100 px-1 text-xs">PRECIOCOSTO</code>,
				<code class="rounded bg-slate-100 px-1 text-xs">CATEGORIA</code>,
				<code class="rounded bg-slate-100 px-1 text-xs">UNIDAD</code>. Opcionales: CODIGOBARRAS y
				<code class="rounded bg-slate-100 px-1 text-xs">SEVENDEPESO</code> (Sí/No: para granel por kg, g, lt, ml).
				Omite los que ya existen por código. Si a algún renglón le falta un dato, el archivo se rechaza y te dice cuál.
			</p>
			<button
				type="button"
				onclick={() =>
					descargarPlantillaCSV(
						'agregar_productos',
						['PRODUCTO', 'CODIGOBARRAS', 'PRECIOVENTA', 'PRECIOCOSTO', 'CATEGORIA', 'UNIDAD', 'SEVENDEPESO'],
						['COCA COLA 600ML', '7501055300013', '18', '12.50', 'BEBIDAS', 'PIEZA', 'No']
					)}
				class="mb-2 text-sm font-medium text-indigo-600 hover:underline"
			>
				⬇️ Descargar plantilla .csv
			</button>
			<input type="file" accept=".xlsx,.csv" onchange={onArchivoProductos} class="mb-2 block w-full text-sm text-slate-600 file:mr-3 file:rounded-lg file:border-0 file:bg-indigo-50 file:px-3 file:py-2 file:text-sm file:font-medium file:text-indigo-700 hover:file:bg-indigo-100" />

			{#if prodErrores.length}
				<p class="mt-2 text-sm font-medium text-red-800">⚠️ Archivo no importado. Corrige estos datos en el Excel y vuelve a subirlo:</p>
			{/if}
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

		<!-- INICIAR INVENTARIO (carga inicial) -->
		<section class="rounded-2xl border-2 border-amber-300 bg-amber-50 p-5 shadow-sm">
			<h2 class="mb-1 font-semibold text-amber-900">🏁 Iniciar inventario desde CSV</h2>
			<p class="mb-2 text-sm text-amber-800">
				Úsalo <strong>solo al inaugurar</strong> el sistema para cargar de golpe lo que ya tienes en
				existencia. Columnas <strong>obligatorias</strong>:
				<code class="rounded bg-amber-100 px-1 text-xs">PRODUCTO</code>,
				<code class="rounded bg-amber-100 px-1 text-xs">PRECIOVENTA</code>,
				<code class="rounded bg-amber-100 px-1 text-xs">PRECIOCOSTO</code>,
				<code class="rounded bg-amber-100 px-1 text-xs">EXISTENCIA</code>,
				<code class="rounded bg-amber-100 px-1 text-xs">CATEGORIA</code>,
				<code class="rounded bg-amber-100 px-1 text-xs">UNIDAD</code>. Opcionales: CODIGOBARRAS y
				<code class="rounded bg-amber-100 px-1 text-xs">SEVENDEPESO</code> (Sí/No: granel por kg, g, lt, ml).
			</p>
			<div class="mb-3 rounded-lg bg-amber-100 px-3 py-2 text-sm text-amber-900">
				⚠️ <strong>Resetea el stock</strong> de cada producto del archivo: borra sus lotes y capas de
				costo actuales y los vuelve a crear con la EXISTENCIA y el PRECIOCOSTO del archivo. Los
				productos que <em>no</em> estén en el archivo no se tocan. Después de inaugurar, el stock entra
				por <strong>Compras</strong> — no vuelvas a usar este botón.
			</div>
			<button
				type="button"
				onclick={() =>
					descargarPlantillaCSV(
						'inventario_inicial',
						['PRODUCTO', 'CODIGOBARRAS', 'PRECIOVENTA', 'PRECIOCOSTO', 'EXISTENCIA', 'CATEGORIA', 'UNIDAD', 'SEVENDEPESO'],
						['COCA COLA 600ML', '7501055300013', '18', '12.50', '24', 'BEBIDAS', 'PIEZA', 'No']
					)}
				class="mb-2 text-sm font-medium text-amber-800 hover:underline"
			>
				⬇️ Descargar plantilla .csv
			</button>
			<input type="file" accept=".xlsx,.csv" onchange={onArchivoIniciar} class="mb-2 block w-full text-sm text-amber-900 file:mr-3 file:rounded-lg file:border-0 file:bg-amber-200 file:px-3 file:py-2 file:text-sm file:font-medium file:text-amber-900 hover:file:bg-amber-300" />

			{#if iniErrores.length}
				<p class="mt-2 text-sm font-medium text-red-800">⚠️ Archivo no cargado. Corrige estos datos en el Excel y vuelve a subirlo:</p>
			{/if}
			{#each iniErrores as err, i (i)}
				<p class="mt-1 text-sm text-red-700">{err}</p>
			{/each}

			{#if iniFilas.length > 0}
				<div class="mt-3 space-y-2 rounded-lg bg-white/70 px-3 py-3 text-sm">
					<p class="text-amber-900">{iniArchivo}: <strong>{iniFilas.length}</strong> productos listos para sembrar.</p>
					<label class="flex items-start gap-2 text-amber-900">
						<input type="checkbox" bind:checked={iniConfirmo} class="mt-0.5 rounded border-amber-400 text-amber-600 focus:ring-amber-500" />
						<span>Entiendo que esto <strong>reemplaza</strong> el stock de estos productos. Solo lo uso al inaugurar.</span>
					</label>
					<button onclick={iniciarInventario} disabled={iniCargando || !iniConfirmo} class="rounded-lg bg-amber-600 px-4 py-1.5 text-sm font-semibold text-white hover:bg-amber-700 disabled:opacity-50">
						{iniCargando ? 'Cargando…' : 'Iniciar inventario'}
					</button>
				</div>
			{/if}

			{#if iniResultado}
				<div class="mt-3 rounded-lg bg-green-50 px-3 py-2 text-sm text-green-800">
					✓ Cargados: {iniResultado.insertados} · Omitidos: {iniResultado.omitidos}
					{#if iniResultado.errores.length}
						<ul class="mt-1 list-inside list-disc text-red-700">
							{#each iniResultado.errores.slice(0, 10) as e, i (i)}<li>{e}</li>{/each}
						</ul>
					{/if}
				</div>
			{/if}
		</section>

		<!-- ACTUALIZAR PRECIOS -->
		<section class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
			<h2 class="mb-1 font-semibold text-slate-800">💲 Actualizar precios (en masa)</h2>
			<p class="mb-3 text-sm text-slate-500">
				Cambia el <strong>precio de venta</strong> de muchos productos a la vez, sin tocar existencias ni
				PEPS. Lo más fácil: <strong>Exportar → Productos</strong>, cambia la columna
				<code class="rounded bg-slate-100 px-1 text-xs">PRECIOVENTA</code> en Excel y sube el archivo
				aquí. Obligatorias: <code class="rounded bg-slate-100 px-1 text-xs">PRODUCTO</code>,
				<code class="rounded bg-slate-100 px-1 text-xs">PRECIOVENTA</code>. Si incluyes
				<code class="rounded bg-slate-100 px-1 text-xs">PRECIOCOSTO</code> también se actualiza el costo de
				referencia. Empata por nombre o CODIGOBARRAS; los que no existan se reportan (no se crean).
			</p>
			<button
				type="button"
				onclick={() =>
					descargarPlantillaCSV(
						'actualizar_precios',
						['PRODUCTO', 'CODIGOBARRAS', 'PRECIOVENTA', 'PRECIOCOSTO'],
						['COCA COLA 600ML', '7501055300013', '19', '13']
					)}
				class="mb-2 text-sm font-medium text-indigo-600 hover:underline"
			>
				⬇️ Descargar plantilla .csv
			</button>
			<input type="file" accept=".xlsx,.csv" onchange={onArchivoPrecios} class="mb-2 block w-full text-sm text-slate-600 file:mr-3 file:rounded-lg file:border-0 file:bg-indigo-50 file:px-3 file:py-2 file:text-sm file:font-medium file:text-indigo-700 hover:file:bg-indigo-100" />

			{#if preErrores.length}
				<p class="mt-2 text-sm font-medium text-red-800">⚠️ Archivo no aplicado. Corrige estos datos y vuelve a subirlo:</p>
			{/if}
			{#each preErrores as err, i (i)}
				<p class="mt-1 text-sm text-red-700">{err}</p>
			{/each}

			{#if preFilas.length > 0}
				<div class="mt-3 flex items-center justify-between rounded-lg bg-slate-50 px-3 py-2 text-sm">
					<span class="text-slate-600">{preArchivo}: <strong>{preFilas.length}</strong> precios listos</span>
					<button onclick={actualizarPrecios} disabled={preCargando} class="rounded-lg bg-indigo-600 px-4 py-1.5 text-sm font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
						{preCargando ? 'Aplicando…' : 'Actualizar precios'}
					</button>
				</div>
			{/if}

			{#if preResultado}
				<div class="mt-3 rounded-lg bg-green-50 px-3 py-2 text-sm text-green-800">
					✓ Actualizados: {preResultado.insertados} · No encontrados: {preResultado.omitidos}
					{#if preResultado.errores.length}
						<ul class="mt-1 list-inside list-disc text-red-700">
							{#each preResultado.errores.slice(0, 10) as e, i (i)}<li>{e}</li>{/each}
						</ul>
					{/if}
				</div>
			{/if}
		</section>

		<!-- IMPORTAR CLIENTES -->
		<section class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
			<h2 class="mb-1 font-semibold text-slate-800">Importar clientes</h2>
			<p class="mb-3 text-sm text-slate-500">
				Archivo .xlsx o .csv. Obligatorias: <code class="rounded bg-slate-100 px-1 text-xs">NOMBRE</code>,
				<code class="rounded bg-slate-100 px-1 text-xs">TELEFONO</code>. Opcional:
				<code class="rounded bg-slate-100 px-1 text-xs">EMAIL</code>.
			</p>
			<button type="button" onclick={() => descargarPlantillaCSV('clientes', ['NOMBRE', 'TELEFONO', 'EMAIL'], ['JUAN PEREZ', '5512345678', 'juan@correo.com'])} class="mb-2 text-sm font-medium text-indigo-600 hover:underline">⬇️ Descargar plantilla .csv</button>
			<input type="file" accept=".xlsx,.csv" onchange={onArchivoClientes} class="mb-2 block w-full text-sm text-slate-600 file:mr-3 file:rounded-lg file:border-0 file:bg-indigo-50 file:px-3 file:py-2 file:text-sm file:font-medium file:text-indigo-700 hover:file:bg-indigo-100" />

			{#if cliErrores.length}
				<p class="mt-2 text-sm font-medium text-red-800">⚠️ Archivo no importado. Corrige estos datos y vuelve a subirlo:</p>
			{/if}
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
			<button type="button" onclick={() => descargarPlantillaCSV('categorias', ['CATEGORIA', 'DESCRIPCION'], ['BEBIDAS', 'Refrescos y aguas'])} class="mb-2 text-sm font-medium text-indigo-600 hover:underline">⬇️ Descargar plantilla .csv</button>
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
				Archivo .xlsx o .csv. Obligatorias: <code class="rounded bg-slate-100 px-1 text-xs">PROVEEDOR</code>,
				<code class="rounded bg-slate-100 px-1 text-xs">TELEFONO</code>. Opcionales: CONTACTO,
				<code class="rounded bg-slate-100 px-1 text-xs">EMAIL</code>. Se omiten los que ya existen.
			</p>
			<button type="button" onclick={() => descargarPlantillaCSV('proveedores', ['PROVEEDOR', 'CONTACTO', 'TELEFONO', 'EMAIL'], ['REFRESCOS SA', 'MARIA LOPEZ', '5512345678', 'ventas@refrescos.com'])} class="mb-2 text-sm font-medium text-indigo-600 hover:underline">⬇️ Descargar plantilla .csv</button>
			<input type="file" accept=".xlsx,.csv" onchange={onArchivoProveedores} class="mb-2 block w-full text-sm text-slate-600 file:mr-3 file:rounded-lg file:border-0 file:bg-indigo-50 file:px-3 file:py-2 file:text-sm file:font-medium file:text-indigo-700 hover:file:bg-indigo-100" />

			{#if provErrores.length}
				<p class="mt-2 text-sm font-medium text-red-800">⚠️ Archivo no importado. Corrige estos datos y vuelve a subirlo:</p>
			{/if}
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
