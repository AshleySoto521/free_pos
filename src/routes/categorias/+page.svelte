<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, type Categoria } from '$lib/api';
	import { session } from '$lib/stores/session';
	import SelectorCatalogo from '$lib/components/SelectorCatalogo.svelte';

	let categorias = $state<Categoria[]>([]);
	let cargando = $state(true);
	let verInactivos = $state(false);
	let modalCatalogo = $state(false);

	let modal = $state(false);
	let editandoId = $state<number | null>(null);
	let nombre = $state('');
	let descripcion = $state('');
	let activo = $state(true);
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
			categorias = await api.listarCategorias(verInactivos);
		} finally {
			cargando = false;
		}
	}

	function abrirNuevo() {
		editandoId = null;
		nombre = '';
		descripcion = '';
		activo = true;
		error = '';
		modal = true;
	}

	function abrirEditar(c: Categoria) {
		editandoId = c.idCategoria;
		nombre = c.categoria;
		descripcion = c.descripcion ?? '';
		activo = c.activo;
		error = '';
		modal = true;
	}

	async function guardar() {
		if (!nombre.trim()) {
			error = 'El nombre es obligatorio.';
			return;
		}
		guardando = true;
		error = '';
		try {
			if (editandoId != null) {
				await api.actualizarCategoria(editandoId, nombre.trim(), descripcion.trim() || null, activo);
			} else {
				await api.crearCategoria(nombre.trim(), descripcion.trim() || null);
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
			<button onclick={() => goto(resolve('/inventario'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
			<h1 class="text-lg font-semibold text-slate-800">Categorías</h1>
		</div>
		<div class="flex gap-2">
			<button onclick={() => (modalCatalogo = true)} class="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium text-slate-600 hover:bg-slate-50">
				📚 Catálogo por giro
			</button>
			<button onclick={abrirNuevo} class="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700">
				+ Nueva categoría
			</button>
		</div>
	</header>

	<main class="mx-auto max-w-4xl p-6">
		<label class="mb-4 flex items-center gap-2 text-sm text-slate-600">
			<input type="checkbox" bind:checked={verInactivos} onchange={cargar} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
			Ver inactivas
		</label>

		{#if cargando}
			<p class="py-10 text-center text-slate-400">Cargando…</p>
		{:else if categorias.length === 0}
			<div class="rounded-xl border border-dashed border-slate-300 bg-white py-12 text-center">
				<p class="text-slate-500">Aún no hay categorías.</p>
				<p class="mt-1 text-sm text-slate-400">Elige un catálogo según tu giro para arrancar rápido.</p>
				<div class="mt-3 flex justify-center gap-2">
					<button onclick={() => (modalCatalogo = true)} class="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700">
						📚 Elegir catálogo por giro
					</button>
					<button onclick={abrirNuevo} class="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium text-slate-600 hover:bg-slate-50">
						Crear una manual
					</button>
				</div>
			</div>
		{:else}
			<div class="overflow-x-auto rounded-xl border border-slate-200 bg-white">
				<table class="w-full text-sm">
					<thead class="bg-slate-50 text-left text-xs uppercase tracking-wide text-slate-500">
						<tr>
							<th class="px-4 py-2.5 font-medium">Categoría</th>
							<th class="px-4 py-2.5 font-medium">Descripción</th>
							<th class="px-4 py-2.5 text-right font-medium">Acciones</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-slate-100">
						{#each categorias as c (c.idCategoria)}
							<tr class="hover:bg-slate-50 {c.activo ? '' : 'opacity-50'}">
								<td class="px-4 py-2.5 font-medium text-slate-800">
									{c.categoria}
									{#if !c.activo}<span class="ml-1 rounded bg-slate-200 px-1.5 py-0.5 text-[10px] font-medium text-slate-500">inactiva</span>{/if}
								</td>
								<td class="px-4 py-2.5 text-slate-600">{c.descripcion ?? '—'}</td>
								<td class="px-4 py-2.5 text-right">
									<button onclick={() => abrirEditar(c)} class="rounded-lg border border-indigo-200 bg-indigo-50 px-2.5 py-1 text-xs font-medium text-indigo-700 hover:bg-indigo-100">Editar</button>
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
		<div class="w-full max-w-sm rounded-2xl bg-white p-6 shadow-xl">
			<div class="mb-4 flex items-center justify-between">
				<h2 class="text-lg font-semibold text-slate-800">{editandoId != null ? 'Editar categoría' : 'Nueva categoría'}</h2>
				<button onclick={() => (modal = false)} class="text-slate-400 hover:text-slate-600" aria-label="Cerrar">✕</button>
			</div>
			<div class="space-y-3">
				<div>
					<label for="cn" class="mb-1 block text-sm font-medium text-slate-700">Nombre</label>
					<input id="cn" bind:value={nombre} placeholder="Ej. Bebidas" class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
				</div>
				<div>
					<label for="cd" class="mb-1 block text-sm font-medium text-slate-700">Descripción (opcional)</label>
					<input id="cd" bind:value={descripcion} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
				</div>
				{#if editandoId != null}
					<label class="flex items-center gap-2 text-sm text-slate-700">
						<input type="checkbox" bind:checked={activo} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
						Activa
					</label>
				{/if}
				{#if error}
					<p class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>
				{/if}
				<button onclick={guardar} disabled={guardando} class="w-full rounded-lg bg-indigo-600 py-2.5 font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
					{guardando ? 'Guardando…' : editandoId != null ? 'Guardar cambios' : 'Crear categoría'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Selector de catálogo por giro -->
{#if modalCatalogo}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="w-full max-w-2xl rounded-2xl bg-white p-6 shadow-xl">
			<div class="mb-4 flex items-center justify-between">
				<div>
					<h2 class="text-lg font-semibold text-slate-800">Catálogo de categorías por giro</h2>
					<p class="text-sm text-slate-500">Elige el giro de tu negocio para cargar sus categorías típicas.</p>
				</div>
				<button onclick={() => (modalCatalogo = false)} class="text-slate-400 hover:text-slate-600" aria-label="Cerrar">✕</button>
			</div>
			<SelectorCatalogo
				onAplicado={async () => {
					modalCatalogo = false;
					await cargar();
				}}
			/>
		</div>
	</div>
{/if}
