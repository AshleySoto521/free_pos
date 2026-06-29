<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api';
	import { session } from '$lib/stores/session';
	import { pesos, fechaHora } from '$lib/format';
	import { exportarXLSX } from '$lib/xlsx';
	import { exportarPDF } from '$lib/pdf';
	import { toast } from '$lib/stores/toast';

	type Reporte = {
		nombre: string;
		titulo: string;
		subtitulo: string;
		encabezados: string[];
		filas: (string | number)[][];
	};

	function hoyStr() {
		const d = new Date();
		return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
	}
	const fmt = (f: string) => fechaHora(f);

	let tipo = $state<'ventas' | 'inventario' | 'movimientos' | 'utilidad' | 'cortes'>('ventas');
	let desde = $state(hoyStr());
	let hasta = $state(hoyStr());
	let formato = $state<'xlsx' | 'pdf' | 'ambos'>('ambos');
	let generando = $state(false);
	let mensaje = $state('');
	let error = $state('');

	const conRango = $derived(
		tipo === 'ventas' || tipo === 'cortes' || tipo === 'movimientos' || tipo === 'utilidad'
	);

	onMount(() => {
		if ($session?.rol !== 'Administrador') goto(resolve('/'));
	});

	async function repVentas(): Promise<Reporte> {
		const [resumen, ventas] = await Promise.all([
			api.reporteVentas(desde, hasta),
			api.listarVentas(desde, hasta)
		]);
		const metodos = resumen.metodos.map((m) => `${m.metodoPago}: ${pesos(m.total)}`).join('  ·  ');
		return {
			nombre: `ventas_${desde}_a_${hasta}`,
			titulo: 'Reporte de ventas',
			subtitulo: `${desde} a ${hasta}  ·  Total ${pesos(resumen.total)}  ·  ${resumen.tickets} tickets${metodos ? '  ·  ' + metodos : ''}`,
			encabezados: ['FOLIO', 'FECHA', 'METODO', 'CAJERO', 'CLIENTE', 'TOTAL', 'ESTATUS'],
			filas: ventas.map((v) => [
				v.folio ?? v.idVenta,
				fmt(v.fechaVenta),
				v.metodoPago,
				v.usuario,
				v.cliente ?? '',
				Number(v.total.toFixed(2)),
				v.estatus
			])
		};
	}

	async function repInventario(): Promise<Reporte> {
		const items = await api.reporteInventarioValorizado();
		const valor = items.reduce((s, x) => s + x.valor, 0);
		return {
			nombre: 'inventario_valorizado',
			titulo: 'Inventario valorizado (PEPS)',
			subtitulo: `${items.length} productos  ·  Valor del inventario ${pesos(valor)}`,
			encabezados: ['PRODUCTO', 'EXISTENCIA', 'COSTO UNIT (PEPS)', 'VALOR'],
			filas: items.map((x) => [
				x.producto,
				Number(x.existencia.toFixed(2)),
				Number(x.costoUnitario.toFixed(2)),
				Number(x.valor.toFixed(2))
			])
		};
	}

	async function repUtilidad(): Promise<Reporte> {
		const items = await api.reporteUtilidad(desde, hasta);
		const ventas = items.reduce((s, x) => s + x.ventas, 0);
		const costo = items.reduce((s, x) => s + x.costo, 0);
		const utilidad = ventas - costo;
		const margen = ventas > 0 ? (utilidad / ventas) * 100 : 0;
		return {
			nombre: `utilidad_${desde}_a_${hasta}`,
			titulo: 'Utilidad (PEPS)',
			subtitulo: `${desde} a ${hasta}  ·  Vendiste ${pesos(ventas)}  ·  Te costó ${pesos(costo)}  ·  Ganaste ${pesos(utilidad)} (${margen.toFixed(1)}%)`,
			encabezados: ['PRODUCTO', 'VENDIDO', 'VENTAS', 'COSTO', 'UTILIDAD', 'MARGEN %'],
			filas: items.map((x) => [
				x.producto,
				Number(x.vendido.toFixed(2)),
				Number(x.ventas.toFixed(2)),
				Number(x.costo.toFixed(2)),
				Number(x.utilidad.toFixed(2)),
				Number(x.margen.toFixed(1))
			])
		};
	}

	async function repCortes(): Promise<Reporte> {
		const cortes = await api.listarCortes(desde, hasta);
		return {
			nombre: `cortes_${desde}_a_${hasta}`,
			titulo: 'Reporte de cortes de caja',
			subtitulo: `${desde} a ${hasta}  ·  ${cortes.length} cortes`,
			encabezados: ['FECHA', 'CAJERO', 'INICIAL', 'ESPERADO', 'CONTADO', 'DIFERENCIA', 'ESTATUS'],
			filas: cortes.map((c) => [
				fmt(c.fechaApertura),
				c.usuario,
				Number(c.montoInicial.toFixed(2)),
				c.montoEsperado ?? '',
				c.montoContado ?? '',
				c.diferencia ?? '',
				c.estatus
			])
		};
	}

	async function repMovimientos(): Promise<Reporte> {
		const m = await api.reporteMovimientos(desde, hasta);
		const tot = (k: 'comprado' | 'vendido') => m.reduce((s, x) => s + x[k], 0);
		return {
			nombre: `movimientos_${desde}_a_${hasta}`,
			titulo: 'Entradas y salidas de mercancía',
			subtitulo: `${desde} a ${hasta}  ·  Comprado ${tot('comprado')}  ·  Vendido ${tot('vendido')}`,
			encabezados: ['PRODUCTO', 'COMPRADO', 'VENDIDO', 'MERMA', 'ENTRADAS', 'SALIDAS'],
			filas: m.map((x) => [
				x.producto,
				Number(x.comprado.toFixed(2)),
				Number(x.vendido.toFixed(2)),
				Number(x.merma.toFixed(2)),
				Number(x.entradas.toFixed(2)),
				Number(x.salidas.toFixed(2))
			])
		};
	}

	async function generar() {
		generando = true;
		error = '';
		mensaje = '';
		try {
			const rep =
				tipo === 'ventas'
					? await repVentas()
					: tipo === 'inventario'
						? await repInventario()
						: tipo === 'movimientos'
							? await repMovimientos()
							: tipo === 'utilidad'
								? await repUtilidad()
								: await repCortes();

			if (rep.filas.length === 0) {
				error = 'No hay datos para ese reporte en el rango elegido.';
				return;
			}
			if (formato === 'xlsx' || formato === 'ambos') {
				exportarXLSX(rep.nombre, rep.encabezados, rep.filas);
			}
			if (formato === 'pdf' || formato === 'ambos') {
				exportarPDF(rep);
			}
			mensaje = `Reporte generado (${rep.filas.length} filas) ✓`;
			toast('Descarga realizada');
		} catch (e) {
			error = String(e);
			toast('No se pudo generar el reporte', 'error');
		} finally {
			generando = false;
		}
	}
</script>

<div class="flex min-h-screen flex-col bg-slate-100">
	<header class="flex items-center gap-3 border-b border-slate-200 bg-white px-5 py-3">
		<button onclick={() => goto(resolve('/'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
		<h1 class="text-lg font-semibold text-slate-800">Reportes</h1>
	</header>

	<main class="mx-auto flex w-full max-w-xl flex-1 flex-col justify-center p-6">
		<div class="space-y-5 rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
			<!-- Tipo de reporte -->
			<div>
				<p class="mb-2 text-sm font-medium text-slate-700">Reporte</p>
				<div class="grid grid-cols-3 gap-2">
					{#each [['ventas', '🛒 Ventas'], ['inventario', '📦 Inventario'], ['movimientos', '🔁 Movimientos'], ['utilidad', '💰 Utilidad'], ['cortes', '💵 Cortes']] as [val, label] (val)}
						<button
							onclick={() => (tipo = val as typeof tipo)}
							class="rounded-lg border px-3 py-2 text-sm font-medium transition {tipo === val ? 'border-indigo-500 bg-indigo-50 text-indigo-700' : 'border-slate-200 text-slate-600 hover:bg-slate-50'}"
						>
							{label}
						</button>
					{/each}
				</div>
			</div>

			<!-- Rango de fechas -->
			{#if conRango}
				<div class="grid grid-cols-2 gap-3">
					<div>
						<label for="d" class="mb-1 block text-sm font-medium text-slate-700">Desde</label>
						<input id="d" type="date" bind:value={desde} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					</div>
					<div>
						<label for="h" class="mb-1 block text-sm font-medium text-slate-700">Hasta</label>
						<input id="h" type="date" bind:value={hasta} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					</div>
				</div>
			{/if}

			<!-- Formato -->
			<div>
				<p class="mb-2 text-sm font-medium text-slate-700">Formato</p>
				<div class="grid grid-cols-3 gap-2">
					{#each [['xlsx', 'Excel (XLSX)'], ['pdf', 'PDF'], ['ambos', 'Ambos']] as [val, label] (val)}
						<button
							onclick={() => (formato = val as typeof formato)}
							class="rounded-lg border px-3 py-2 text-sm font-medium transition {formato === val ? 'border-indigo-500 bg-indigo-50 text-indigo-700' : 'border-slate-200 text-slate-600 hover:bg-slate-50'}"
						>
							{label}
						</button>
					{/each}
				</div>
			</div>

			<button onclick={generar} disabled={generando} class="w-full rounded-lg bg-indigo-600 py-2.5 font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
				{generando ? 'Generando…' : 'Generar reporte'}
			</button>

			{#if error}<p class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>{/if}
			{#if mensaje}<p class="rounded-lg bg-green-50 px-3 py-2 text-sm text-green-700">{mensaje}</p>{/if}
		</div>
	</main>
</div>
