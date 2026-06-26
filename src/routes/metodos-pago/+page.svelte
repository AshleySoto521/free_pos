<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, type MetodoPago } from '$lib/api';
	import { session } from '$lib/stores/session';

	let metodos = $state<MetodoPago[]>([]);
	let cargando = $state(true);
	let verInactivos = $state(false);

	let modal = $state(false);
	let editandoId = $state<number | null>(null);
	let nombre = $state('');
	let requiereReferencia = $state(false);
	let activo = $state(true);
	let guardando = $state(false);
	let error = $state('');

	// Estos dos los usa la lógica del sistema por nombre; mejor no renombrarlos.
	const especiales = ['Efectivo', 'Fiado'];

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
			metodos = await api.listarMetodosPago(verInactivos);
		} finally {
			cargando = false;
		}
	}

	function abrirNuevo() {
		editandoId = null;
		nombre = '';
		requiereReferencia = false;
		activo = true;
		error = '';
		modal = true;
	}

	function abrirEditar(m: MetodoPago) {
		editandoId = m.idMetodoPago;
		nombre = m.metodoPago;
		requiereReferencia = m.requiereReferencia;
		activo = m.activo;
		error = '';
		modal = true;
	}

	async function guardar() {
		if (!nombre.trim()) {
			error = 'Escribe un nombre.';
			return;
		}
		guardando = true;
		error = '';
		try {
			if (editandoId != null) {
				await api.actualizarMetodoPago(editandoId, nombre.trim(), requiereReferencia, activo);
			} else {
				await api.crearMetodoPago(nombre.trim(), requiereReferencia);
			}
			await cargar();
			modal = false;
		} catch (e) {
			error = String(e);
		} finally {
			guardando = false;
		}
	}
</script>

<div class="min-h-screen bg-slate-100">
	<header class="flex items-center justify-between border-b border-slate-200 bg-white px-5 py-3">
		<div class="flex items-center gap-3">
			<button onclick={() => goto(resolve('/catalogos'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
			<h1 class="text-lg font-semibold text-slate-800">Métodos de pago</h1>
		</div>
		<button onclick={abrirNuevo} class="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700">
			+ Nuevo método
		</button>
	</header>

	<main class="mx-auto max-w-3xl p-6">
		<label class="mb-4 flex items-center gap-2 text-sm text-slate-600">
			<input type="checkbox" bind:checked={verInactivos} onchange={cargar} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
			Ver inactivos
		</label>

		{#if cargando}
			<p class="py-10 text-center text-slate-400">Cargando…</p>
		{:else}
			<div class="overflow-x-auto rounded-xl border border-slate-200 bg-white">
				<table class="w-full text-sm">
					<thead class="bg-slate-50 text-left text-xs uppercase tracking-wide text-slate-500">
						<tr>
							<th class="px-4 py-2.5 font-medium">Método</th>
							<th class="px-4 py-2.5 font-medium">Referencia</th>
							<th class="px-4 py-2.5 font-medium">Estado</th>
							<th class="px-4 py-2.5 text-right font-medium">Acciones</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-slate-100">
						{#each metodos as m (m.idMetodoPago)}
							<tr class="hover:bg-slate-50 {m.activo ? '' : 'opacity-50'}">
								<td class="px-4 py-2.5 font-medium text-slate-800">
									{m.metodoPago}
									{#if especiales.includes(m.metodoPago)}<span class="ml-1 rounded bg-amber-100 px-1.5 py-0.5 text-[10px] font-medium text-amber-700">sistema</span>{/if}
								</td>
								<td class="px-4 py-2.5 text-slate-600">{m.requiereReferencia ? 'Sí' : 'No'}</td>
								<td class="px-4 py-2.5">
									{#if m.activo}<span class="text-xs text-green-600">Activo</span>{:else}<span class="text-xs text-slate-400">Inactivo</span>{/if}
								</td>
								<td class="px-4 py-2.5 text-right">
									<button onclick={() => abrirEditar(m)} class="rounded-lg border border-indigo-200 bg-indigo-50 px-2.5 py-1 text-xs font-medium text-indigo-700 hover:bg-indigo-100">Editar</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
			<p class="mt-3 text-xs text-slate-400">
				Nota: "Efectivo" y "Fiado" los usa el sistema por su nombre (corte de caja y fiado). Evita
				renombrarlos o desactivarlos.
			</p>
		{/if}
	</main>
</div>

{#if modal}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="w-full max-w-xs rounded-2xl bg-white p-6 shadow-xl">
			<h2 class="mb-3 text-base font-semibold text-slate-800">{editandoId != null ? 'Editar método' : 'Nuevo método'}</h2>
			<input bind:value={nombre} placeholder="Ej. Vales de despensa" class="mb-3 w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
			<label class="mb-2 flex items-center gap-2 text-sm text-slate-700">
				<input type="checkbox" bind:checked={requiereReferencia} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
				Requiere referencia (folio/voucher)
			</label>
			{#if editandoId != null}
				<label class="mb-3 flex items-center gap-2 text-sm text-slate-700">
					<input type="checkbox" bind:checked={activo} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
					Activo
				</label>
			{/if}
			{#if error}<p class="mb-2 text-sm text-red-700">{error}</p>{/if}
			<div class="flex gap-2">
				<button onclick={() => (modal = false)} class="flex-1 rounded-lg border border-slate-300 py-2 text-sm text-slate-600 hover:bg-slate-50">Cancelar</button>
				<button onclick={guardar} disabled={guardando} class="flex-1 rounded-lg bg-indigo-600 py-2 text-sm font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
					{guardando ? 'Guardando…' : 'Guardar'}
				</button>
			</div>
		</div>
	</div>
{/if}
