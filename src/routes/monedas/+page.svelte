<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, type Moneda, type Denominacion } from '$lib/api';
	import { session } from '$lib/stores/session';
	import { toast } from '$lib/stores/toast';

	let monedas = $state<Moneda[]>([]);
	let cargando = $state(true);

	// Modal moneda
	let modal = $state(false);
	let editandoId = $state<number | null>(null);
	let moneda = $state('');
	let codigo = $state('');
	let simbolo = $state('');
	let esPrincipal = $state(false);
	let activo = $state(true);
	let guardando = $state(false);
	let error = $state('');

	// Modal denominaciones
	let modalDenom = $state(false);
	let denomMoneda = $state<Moneda | null>(null);
	let denoms = $state<Denominacion[]>([]);
	let nuevoValor = $state<number | null>(null);
	let nuevoTipo = $state<'Billete' | 'Moneda'>('Billete');
	let errorDenom = $state('');

	onMount(() => {
		if ($session?.rol !== 'Administrador') {
			goto(resolve('/'));
			return;
		}
		cargar();
	});

	async function cargar() {
		cargando = true;
		try {
			monedas = await api.listarMonedas(true);
		} finally {
			cargando = false;
		}
	}

	function abrirNueva() {
		editandoId = null;
		moneda = '';
		codigo = '';
		simbolo = '$';
		esPrincipal = false;
		activo = true;
		error = '';
		modal = true;
	}

	function abrirEditar(m: Moneda) {
		editandoId = m.idMoneda;
		moneda = m.moneda;
		codigo = m.codigo;
		simbolo = m.simbolo ?? '';
		esPrincipal = m.esPrincipal;
		activo = m.activo;
		error = '';
		modal = true;
	}

	async function guardarMoneda() {
		if (!moneda.trim() || !codigo.trim()) {
			error = 'Nombre y código son obligatorios.';
			return;
		}
		guardando = true;
		error = '';
		try {
			if (editandoId != null) {
				await api.actualizarMoneda(editandoId, moneda.trim(), codigo.trim().toUpperCase(), simbolo.trim() || null, esPrincipal, activo);
			} else {
				await api.crearMoneda(moneda.trim(), codigo.trim().toUpperCase(), simbolo.trim() || null, esPrincipal);
			}
			await cargar();
			modal = false;
			toast('Moneda guardada');
		} catch (e) {
			error = String(e);
		} finally {
			guardando = false;
		}
	}

	async function abrirDenoms(m: Moneda) {
		denomMoneda = m;
		nuevoValor = null;
		nuevoTipo = 'Billete';
		errorDenom = '';
		modalDenom = true;
		denoms = await api.listarDenominaciones(m.idMoneda);
	}

	async function agregarDenom() {
		if (!denomMoneda || !nuevoValor || nuevoValor <= 0) {
			errorDenom = 'Escribe un valor mayor a cero.';
			return;
		}
		try {
			await api.crearDenominacion(denomMoneda.idMoneda, nuevoValor, nuevoTipo);
			denoms = await api.listarDenominaciones(denomMoneda.idMoneda);
			nuevoValor = null;
		} catch (e) {
			errorDenom = String(e);
		}
	}

	async function eliminarDenom(d: Denominacion) {
		if (!denomMoneda) return;
		await api.eliminarDenominacion(d.idDenominacion);
		denoms = await api.listarDenominaciones(denomMoneda.idMoneda);
	}
</script>

<div class="min-h-screen bg-slate-100">
	<header class="flex items-center justify-between border-b border-slate-200 bg-white px-5 py-3">
		<div class="flex items-center gap-3">
			<button onclick={() => goto(resolve('/catalogos'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
			<h1 class="text-lg font-semibold text-slate-800">Monedas</h1>
		</div>
		<button onclick={abrirNueva} class="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700">+ Nueva moneda</button>
	</header>

	<main class="mx-auto max-w-4xl p-6">
		{#if cargando}
			<p class="py-10 text-center text-slate-400">Cargando…</p>
		{:else}
			<div class="overflow-x-auto rounded-xl border border-slate-200 bg-white">
				<table class="w-full text-sm">
					<thead class="bg-slate-50 text-left text-xs uppercase tracking-wide text-slate-500">
						<tr>
							<th class="px-4 py-2.5 font-medium">Moneda</th>
							<th class="px-4 py-2.5 font-medium">Código</th>
							<th class="px-4 py-2.5 font-medium">Símbolo</th>
							<th class="px-4 py-2.5 text-right font-medium">Acciones</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-slate-100">
						{#each monedas as m (m.idMoneda)}
							<tr class="hover:bg-slate-50 {m.activo ? '' : 'opacity-50'}">
								<td class="px-4 py-2.5 font-medium text-slate-800">
									{m.moneda}
									{#if m.esPrincipal}<span class="ml-1 rounded bg-indigo-100 px-1.5 py-0.5 text-[10px] font-medium text-indigo-700">principal</span>{/if}
									{#if !m.activo}<span class="ml-1 rounded bg-slate-200 px-1.5 py-0.5 text-[10px] font-medium text-slate-500">inactiva</span>{/if}
								</td>
								<td class="px-4 py-2.5 text-slate-600">{m.codigo}</td>
								<td class="px-4 py-2.5 text-slate-600">{m.simbolo ?? '—'}</td>
								<td class="px-4 py-2.5 text-right whitespace-nowrap">
									<button onclick={() => abrirDenoms(m)} class="rounded-lg border border-slate-200 px-2.5 py-1 text-xs font-medium text-slate-600 hover:bg-slate-50">Denominaciones</button>
									<button onclick={() => abrirEditar(m)} class="rounded-lg border border-indigo-200 bg-indigo-50 px-2.5 py-1 text-xs font-medium text-indigo-700 hover:bg-indigo-100">Editar</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</main>
</div>

<!-- Modal moneda -->
{#if modal}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="w-full max-w-sm rounded-2xl bg-white p-6 shadow-xl">
			<div class="mb-4 flex items-center justify-between">
				<h2 class="text-lg font-semibold text-slate-800">{editandoId != null ? 'Editar moneda' : 'Nueva moneda'}</h2>
				<button onclick={() => (modal = false)} class="text-slate-400 hover:text-slate-600" aria-label="Cerrar">✕</button>
			</div>
			<div class="space-y-3">
				<div>
					<label for="mn" class="mb-1 block text-sm font-medium text-slate-700">Nombre</label>
					<input id="mn" bind:value={moneda} placeholder="Ej. Dólar estadounidense" class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
				</div>
				<div class="grid grid-cols-2 gap-3">
					<div>
						<label for="cod" class="mb-1 block text-sm font-medium text-slate-700">Código</label>
						<input id="cod" bind:value={codigo} placeholder="USD" maxlength="5" class="w-full rounded-lg border-slate-300 uppercase shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					</div>
					<div>
						<label for="sim" class="mb-1 block text-sm font-medium text-slate-700">Símbolo</label>
						<input id="sim" bind:value={simbolo} placeholder="$" class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					</div>
				</div>
				<label class="flex items-center gap-2 text-sm text-slate-700">
					<input type="checkbox" bind:checked={esPrincipal} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
					Moneda principal del negocio
				</label>
				{#if editandoId != null}
					<label class="flex items-center gap-2 text-sm text-slate-700">
						<input type="checkbox" bind:checked={activo} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
						Activa
					</label>
				{/if}
				{#if error}<p class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>{/if}
				<button onclick={guardarMoneda} disabled={guardando} class="w-full rounded-lg bg-indigo-600 py-2.5 font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
					{guardando ? 'Guardando…' : 'Guardar'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Modal denominaciones -->
{#if modalDenom && denomMoneda}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="flex max-h-[85vh] w-full max-w-sm flex-col rounded-2xl bg-white p-6 shadow-xl">
			<div class="mb-3 flex items-center justify-between">
				<h2 class="text-lg font-semibold text-slate-800">Denominaciones · {denomMoneda.codigo}</h2>
				<button onclick={() => (modalDenom = false)} class="text-slate-400 hover:text-slate-600" aria-label="Cerrar">✕</button>
			</div>

			<div class="min-h-0 flex-1 overflow-y-auto">
				{#if denoms.length === 0}
					<p class="py-4 text-center text-sm text-slate-400">Sin denominaciones.</p>
				{:else}
					<ul class="divide-y divide-slate-100">
						{#each denoms as d (d.idDenominacion)}
							<li class="flex items-center justify-between py-2 text-sm">
								<span class="text-slate-700">{denomMoneda.simbolo ?? '$'}{d.valor} <span class="text-xs text-slate-400">{d.tipo ?? ''}</span></span>
								<button onclick={() => eliminarDenom(d)} class="text-slate-400 hover:text-red-500" aria-label="Eliminar">✕</button>
							</li>
						{/each}
					</ul>
				{/if}
			</div>

			<div class="mt-3 border-t border-slate-100 pt-3">
				<div class="flex gap-2">
					<input type="number" step="0.01" min="0" bind:value={nuevoValor} placeholder="Valor" class="w-24 rounded-lg border-slate-300 text-sm shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					<select bind:value={nuevoTipo} class="rounded-lg border-slate-300 text-sm shadow-sm focus:border-indigo-500 focus:ring-indigo-500">
						<option value="Billete">Billete</option>
						<option value="Moneda">Moneda</option>
					</select>
					<button onclick={agregarDenom} class="flex-1 rounded-lg bg-indigo-600 text-sm font-semibold text-white hover:bg-indigo-700">Agregar</button>
				</div>
				{#if errorDenom}<p class="mt-2 text-sm text-red-700">{errorDenom}</p>{/if}
			</div>
		</div>
	</div>
{/if}
