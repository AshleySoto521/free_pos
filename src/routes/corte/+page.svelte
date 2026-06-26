<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import {
		api,
		type Corte,
		type ResumenParcialCorte,
		type ResumenCorte,
		type MovimientoCaja
	} from '$lib/api';
	import { session } from '$lib/stores/session';
	import { pesos, fechaHora } from '$lib/format';
	import ContadorMonedas from '$lib/components/ContadorMonedas.svelte';

	let corte = $state<Corte | null>(null);
	let resumen = $state<ResumenParcialCorte | null>(null);
	let movimientos = $state<MovimientoCaja[]>([]);
	let resumenFinal = $state<ResumenCorte | null>(null);
	let cargando = $state(true);

	// Abrir caja
	let montoInicial = $state<number | null>(0);
	let abriendo = $state(false);
	let errorAbrir = $state('');

	// Movimiento
	let modalMov = $state(false);
	let tipoMov = $state<'Ingreso' | 'Retiro'>('Retiro');
	let montoMov = $state<number | null>(null);
	let conceptoMov = $state('');
	let errorMov = $state('');

	// Cerrar
	let modalCerrar = $state(false);
	let montoContado = $state<number | null>(null);
	let cerrando = $state(false);
	let errorCerrar = $state('');

	const diferenciaPreview = $derived((montoContado ?? 0) - (resumen?.montoEsperado ?? 0));

	onMount(cargar);

	async function cargar() {
		cargando = true;
		try {
			const uid = $session?.idUsuario;
			if (!uid) return;
			corte = await api.corteAbierto(uid);
			if (corte) await refrescar();
		} finally {
			cargando = false;
		}
	}

	async function refrescar() {
		if (!corte) return;
		resumen = await api.resumenCorte(corte.idCorte);
		movimientos = await api.listarMovimientosCaja(corte.idCorte);
	}

	async function abrir() {
		if (!$session) return;
		abriendo = true;
		errorAbrir = '';
		try {
			await api.abrirCorte($session.idUsuario, montoInicial ?? 0);
			resumenFinal = null;
			await cargar();
		} catch (e) {
			errorAbrir = String(e);
		} finally {
			abriendo = false;
		}
	}

	function abrirModalMov(tipo: 'Ingreso' | 'Retiro') {
		tipoMov = tipo;
		montoMov = null;
		conceptoMov = '';
		errorMov = '';
		modalMov = true;
	}

	async function guardarMov() {
		if (!corte || !montoMov || montoMov <= 0) {
			errorMov = 'Escribe un monto mayor a cero.';
			return;
		}
		try {
			await api.registrarMovimientoCaja(corte.idCorte, tipoMov, montoMov, conceptoMov.trim() || null);
			await refrescar();
			modalMov = false;
		} catch (e) {
			errorMov = String(e);
		}
	}

	async function cerrar() {
		if (!corte) return;
		cerrando = true;
		errorCerrar = '';
		try {
			resumenFinal = await api.cerrarCorte(corte.idCorte, montoContado ?? 0);
			corte = null;
			resumen = null;
			movimientos = [];
			modalCerrar = false;
		} catch (e) {
			errorCerrar = String(e);
		} finally {
			cerrando = false;
		}
	}

	function nuevoTurno() {
		resumenFinal = null;
		montoInicial = 0;
	}

	const hora = (f: string) => fechaHora(f);
</script>

<div class="min-h-screen bg-slate-100">
	<header class="flex items-center gap-3 border-b border-slate-200 bg-white px-5 py-3">
		<button onclick={() => goto(resolve('/'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
		<h1 class="text-lg font-semibold text-slate-800">Corte de caja</h1>
	</header>

	<main class="mx-auto max-w-2xl p-6">
		{#if cargando}
			<p class="py-10 text-center text-slate-400">Cargando…</p>

		{:else if resumenFinal}
			<!-- Resumen final del cierre -->
			<div class="rounded-2xl border border-slate-200 bg-white p-6 text-center shadow-sm">
				<h2 class="text-lg font-semibold text-slate-800">Caja cerrada</h2>
				<div class="my-4 space-y-1 text-sm text-slate-600">
					<div class="flex justify-between"><span>Efectivo esperado</span><span>{pesos(resumenFinal.montoEsperado)}</span></div>
					<div class="flex justify-between"><span>Efectivo contado</span><span>{pesos(resumenFinal.montoContado)}</span></div>
				</div>
				<div class="rounded-xl p-4 {resumenFinal.diferencia === 0 ? 'bg-green-50 text-green-700' : resumenFinal.diferencia > 0 ? 'bg-amber-50 text-amber-700' : 'bg-red-50 text-red-700'}">
					<p class="text-sm">{resumenFinal.diferencia === 0 ? 'Cuadró exacto' : resumenFinal.diferencia > 0 ? 'Sobrante' : 'Faltante'}</p>
					<p class="text-2xl font-bold">{pesos(Math.abs(resumenFinal.diferencia))}</p>
				</div>
				<button onclick={nuevoTurno} class="mt-5 w-full rounded-lg bg-indigo-600 py-2.5 font-semibold text-white hover:bg-indigo-700">
					Abrir nuevo turno
				</button>
			</div>

		{:else if corte && resumen}
			<!-- Caja abierta -->
			<div class="mb-4 rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
				<div class="mb-4 flex items-center justify-between">
					<div>
						<p class="text-sm text-slate-500">Caja abierta</p>
						<p class="text-xs text-slate-400">Desde {hora(corte.fechaApertura)}</p>
					</div>
					<button onclick={() => { montoContado = null; errorCerrar = ''; modalCerrar = true; }} class="rounded-lg bg-slate-800 px-4 py-2 text-sm font-semibold text-white hover:bg-slate-900">
						Cerrar caja
					</button>
				</div>

				<dl class="space-y-2 text-sm">
					<div class="flex justify-between"><dt class="text-slate-500">Fondo inicial</dt><dd class="font-medium text-slate-700">{pesos(resumen.montoInicial)}</dd></div>
					<div class="flex justify-between"><dt class="text-slate-500">Ventas en efectivo</dt><dd class="font-medium text-slate-700">{pesos(resumen.ventasEfectivo)}</dd></div>
					<div class="flex justify-between"><dt class="text-slate-500">Ingresos</dt><dd class="font-medium text-green-600">+{pesos(resumen.ingresos)}</dd></div>
					<div class="flex justify-between"><dt class="text-slate-500">Retiros</dt><dd class="font-medium text-red-600">−{pesos(resumen.retiros)}</dd></div>
					<div class="flex justify-between border-t border-slate-100 pt-2 text-base"><dt class="font-semibold text-slate-800">Efectivo esperado</dt><dd class="font-bold text-slate-800">{pesos(resumen.montoEsperado)}</dd></div>
				</dl>

				<div class="mt-4 flex gap-2">
					<button onclick={() => abrirModalMov('Ingreso')} class="flex-1 rounded-lg border border-green-200 bg-green-50 py-2 text-sm font-medium text-green-700 hover:bg-green-100">+ Ingreso</button>
					<button onclick={() => abrirModalMov('Retiro')} class="flex-1 rounded-lg border border-red-200 bg-red-50 py-2 text-sm font-medium text-red-700 hover:bg-red-100">− Retiro</button>
				</div>
			</div>

			<!-- Movimientos -->
			<div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
				<h3 class="mb-2 text-sm font-semibold text-slate-700">Movimientos de efectivo</h3>
				{#if movimientos.length === 0}
					<p class="py-4 text-center text-sm text-slate-400">Sin movimientos.</p>
				{:else}
					<ul class="divide-y divide-slate-100">
						{#each movimientos as m (m.idMovimiento)}
							<li class="flex items-center justify-between py-2 text-sm">
								<div>
									<span class="font-medium {m.tipo === 'Ingreso' ? 'text-green-600' : 'text-red-600'}">{m.tipo}</span>
									<span class="text-slate-500">· {m.concepto || 'Sin concepto'}</span>
									<span class="ml-1 text-xs text-slate-400">{hora(m.fecha)}</span>
								</div>
								<span class="font-medium {m.tipo === 'Ingreso' ? 'text-green-600' : 'text-red-600'}">
									{m.tipo === 'Ingreso' ? '+' : '−'}{pesos(m.monto)}
								</span>
							</li>
						{/each}
					</ul>
				{/if}
			</div>

		{:else}
			<!-- Abrir caja -->
			<div class="mx-auto max-w-sm rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
				<h2 class="text-lg font-semibold text-slate-800">Abrir caja</h2>
				<p class="mt-1 mb-4 text-sm text-slate-500">¿Con cuánto efectivo inicias el turno?</p>
				<p class="mb-2 text-left text-sm font-medium text-slate-700">Cuenta el efectivo con el que abres</p>
				<div class="mb-3"><ContadorMonedas onTotal={(n) => (montoInicial = n)} /></div>
				{#if errorAbrir}
					<p class="mb-3 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{errorAbrir}</p>
				{/if}
				<button onclick={abrir} disabled={abriendo} class="w-full rounded-lg bg-indigo-600 py-2.5 font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
					{abriendo ? 'Abriendo…' : 'Abrir caja'}
				</button>
			</div>
		{/if}
	</main>
</div>

<!-- Modal movimiento -->
{#if modalMov}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="w-full max-w-xs rounded-2xl bg-white p-6 shadow-xl">
			<h2 class="mb-3 text-base font-semibold text-slate-800">{tipoMov === 'Ingreso' ? 'Ingreso' : 'Retiro'} de efectivo</h2>
			<label for="mm" class="mb-1 block text-sm font-medium text-slate-700">Monto</label>
			<input id="mm" type="number" step="0.01" min="0" bind:value={montoMov} class="mb-3 w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
			<label for="mc" class="mb-1 block text-sm font-medium text-slate-700">Concepto</label>
			<input id="mc" bind:value={conceptoMov} placeholder="Ej. compra de gas" class="mb-3 w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
			{#if errorMov}
				<p class="mb-2 text-sm text-red-700">{errorMov}</p>
			{/if}
			<div class="flex gap-2">
				<button onclick={() => (modalMov = false)} class="flex-1 rounded-lg border border-slate-300 py-2 text-sm text-slate-600 hover:bg-slate-50">Cancelar</button>
				<button onclick={guardarMov} class="flex-1 rounded-lg bg-indigo-600 py-2 text-sm font-semibold text-white hover:bg-indigo-700">Registrar</button>
			</div>
		</div>
	</div>
{/if}

<!-- Modal cerrar -->
{#if modalCerrar}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="max-h-[90vh] w-full max-w-sm overflow-y-auto rounded-2xl bg-white p-6 shadow-xl">
			<h2 class="mb-1 text-base font-semibold text-slate-800">Cerrar caja</h2>
			<p class="mb-3 text-sm text-slate-500">Cuenta el efectivo en caja y anótalo.</p>
			<div class="mb-3 flex justify-between rounded-lg bg-slate-50 px-3 py-2 text-sm">
				<span class="text-slate-500">Esperado</span><span class="font-semibold text-slate-700">{pesos(resumen?.montoEsperado ?? 0)}</span>
			</div>
			<p class="mb-2 text-sm font-medium text-slate-700">Cuenta el efectivo en caja</p>
			<div class="mb-2"><ContadorMonedas onTotal={(n) => (montoContado = n)} /></div>
			{#if montoContado != null}
				<p class="mb-3 text-right text-sm {diferenciaPreview === 0 ? 'text-green-600' : diferenciaPreview > 0 ? 'text-amber-600' : 'text-red-600'}">
					{diferenciaPreview === 0 ? 'Cuadra' : diferenciaPreview > 0 ? 'Sobra' : 'Falta'} {pesos(Math.abs(diferenciaPreview))}
				</p>
			{/if}
			{#if errorCerrar}
				<p class="mb-2 text-sm text-red-700">{errorCerrar}</p>
			{/if}
			<div class="flex gap-2">
				<button onclick={() => (modalCerrar = false)} class="flex-1 rounded-lg border border-slate-300 py-2 text-sm text-slate-600 hover:bg-slate-50">Cancelar</button>
				<button onclick={cerrar} disabled={cerrando} class="flex-1 rounded-lg bg-slate-800 py-2 text-sm font-semibold text-white hover:bg-slate-900 disabled:opacity-50">
					{cerrando ? 'Cerrando…' : 'Cerrar caja'}
				</button>
			</div>
		</div>
	</div>
{/if}
