<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, type UnidadMedida } from '$lib/api';
	import { session } from '$lib/stores/session';

	let unidades = $state<UnidadMedida[]>([]);
	let cargando = $state(true);

	let modal = $state(false);
	let editandoId = $state<number | null>(null);
	let nombre = $state('');
	let guardando = $state(false);
	let error = $state('');

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
			unidades = await api.listarUnidades();
		} finally {
			cargando = false;
		}
	}

	function abrirNuevo() {
		editandoId = null;
		nombre = '';
		error = '';
		modal = true;
	}

	function abrirEditar(u: UnidadMedida) {
		editandoId = u.idUnidadMedida;
		nombre = u.unidadMedida;
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
				await api.actualizarUnidad(editandoId, nombre.trim());
			} else {
				await api.crearUnidad(nombre.trim());
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
			<h1 class="text-lg font-semibold text-slate-800">Unidades de medida</h1>
		</div>
		<button onclick={abrirNuevo} class="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700">
			+ Nueva unidad
		</button>
	</header>

	<main class="mx-auto max-w-2xl p-6">
		{#if cargando}
			<p class="py-10 text-center text-slate-400">Cargando…</p>
		{:else}
			<div class="overflow-x-auto rounded-xl border border-slate-200 bg-white">
				<table class="w-full text-sm">
					<thead class="bg-slate-50 text-left text-xs uppercase tracking-wide text-slate-500">
						<tr>
							<th class="px-4 py-2.5 font-medium">Unidad</th>
							<th class="px-4 py-2.5 text-right font-medium">Acciones</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-slate-100">
						{#each unidades as u (u.idUnidadMedida)}
							<tr class="hover:bg-slate-50">
								<td class="px-4 py-2.5 font-medium text-slate-800">{u.unidadMedida}</td>
								<td class="px-4 py-2.5 text-right">
									<button onclick={() => abrirEditar(u)} class="rounded-lg border border-indigo-200 bg-indigo-50 px-2.5 py-1 text-xs font-medium text-indigo-700 hover:bg-indigo-100">Editar</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</main>
</div>

{#if modal}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="w-full max-w-xs rounded-2xl bg-white p-6 shadow-xl">
			<h2 class="mb-3 text-base font-semibold text-slate-800">{editandoId != null ? 'Editar unidad' : 'Nueva unidad'}</h2>
			<input bind:value={nombre} placeholder="Ej. Caja" class="mb-2 w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
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
