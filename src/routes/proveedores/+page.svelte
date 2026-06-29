<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, type Proveedor } from '$lib/api';
	import { session } from '$lib/stores/session';

	let proveedores = $state<Proveedor[]>([]);
	let cargando = $state(true);
	let busqueda = $state('');
	let verInactivos = $state(false);

	const filtrados = $derived(
		busqueda.trim()
			? proveedores.filter((p) =>
					(p.proveedor + ' ' + (p.contacto ?? '') + ' ' + (p.telefono ?? ''))
						.toLowerCase()
						.includes(busqueda.toLowerCase())
				)
			: proveedores
	);

	let modal = $state(false);
	let editandoId = $state<number | null>(null);
	let nombre = $state('');
	let contacto = $state('');
	let telefono = $state('');
	let email = $state('');
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
			proveedores = await api.listarProveedores(verInactivos);
		} finally {
			cargando = false;
		}
	}

	function abrirNuevo() {
		editandoId = null;
		nombre = '';
		contacto = '';
		telefono = '';
		email = '';
		activo = true;
		error = '';
		modal = true;
	}

	function abrirEditar(p: Proveedor) {
		editandoId = p.idProveedor;
		nombre = p.proveedor;
		contacto = p.contacto ?? '';
		telefono = p.telefono ?? '';
		email = p.email ?? '';
		activo = p.activo;
		error = '';
		modal = true;
	}

	async function guardar() {
		if (!nombre.trim()) {
			error = 'El nombre es obligatorio.';
			return;
		}
		if (!telefono.trim()) {
			error = 'El teléfono es obligatorio.';
			return;
		}
		guardando = true;
		error = '';
		try {
			if (editandoId != null) {
				await api.actualizarProveedor(editandoId, {
					proveedor: nombre.trim(),
					contacto: contacto.trim() || null,
					telefono: telefono.trim(),
					email: email.trim() || null,
					activo
				});
			} else {
				await api.crearProveedor({
					proveedor: nombre.trim(),
					contacto: contacto.trim() || null,
					telefono: telefono.trim(),
					email: email.trim() || null
				});
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
			<button onclick={() => goto(resolve('/'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
			<h1 class="text-lg font-semibold text-slate-800">Proveedores</h1>
		</div>
		<div class="flex gap-2">
			<button onclick={() => goto(resolve('/datos'))} class="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium text-slate-600 hover:bg-slate-50">
				📤 Importar / Exportar
			</button>
			<button onclick={abrirNuevo} class="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700">
				+ Nuevo proveedor
			</button>
		</div>
	</header>

	<main class="mx-auto max-w-5xl p-6">
		<div class="mb-4 flex items-center gap-4">
			<input
				bind:value={busqueda}
				placeholder="Buscar proveedor…"
				class="w-full max-w-sm rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500"
			/>
			<label class="flex shrink-0 items-center gap-2 text-sm text-slate-600">
				<input type="checkbox" bind:checked={verInactivos} onchange={cargar} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
				Ver inactivos
			</label>
		</div>

		{#if cargando}
			<p class="py-10 text-center text-slate-400">Cargando…</p>
		{:else if proveedores.length === 0}
			<div class="rounded-xl border border-dashed border-slate-300 bg-white py-12 text-center">
				<p class="text-slate-500">Aún no hay proveedores.</p>
				<button onclick={abrirNuevo} class="mt-3 rounded-lg bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700">
					Agregar el primero
				</button>
			</div>
		{:else}
			<div class="overflow-x-auto rounded-xl border border-slate-200 bg-white">
				<table class="w-full text-sm">
					<thead class="bg-slate-50 text-left text-xs uppercase tracking-wide text-slate-500">
						<tr>
							<th class="px-4 py-2.5 font-medium">Proveedor</th>
							<th class="px-4 py-2.5 font-medium">Contacto</th>
							<th class="px-4 py-2.5 font-medium">Teléfono</th>
							<th class="px-4 py-2.5 text-right font-medium">Acciones</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-slate-100">
						{#each filtrados as p (p.idProveedor)}
							<tr class="hover:bg-slate-50 {p.activo ? '' : 'opacity-50'}">
								<td class="px-4 py-2.5 font-medium text-slate-800">
									{p.proveedor}
									{#if !p.activo}<span class="ml-1 rounded bg-slate-200 px-1.5 py-0.5 text-[10px] font-medium text-slate-500">inactivo</span>{/if}
								</td>
								<td class="px-4 py-2.5 text-slate-600">{p.contacto ?? '—'}</td>
								<td class="px-4 py-2.5 text-slate-600">
									{p.telefono ?? '—'}
									{#if p.email}<div class="text-xs text-slate-400">{p.email}</div>{/if}
								</td>
								<td class="px-4 py-2.5 text-right">
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

{#if modal}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="w-full max-w-sm rounded-2xl bg-white p-6 shadow-xl">
			<div class="mb-4 flex items-center justify-between">
				<h2 class="text-lg font-semibold text-slate-800">{editandoId != null ? 'Editar proveedor' : 'Nuevo proveedor'}</h2>
				<button onclick={() => (modal = false)} class="text-slate-400 hover:text-slate-600" aria-label="Cerrar">✕</button>
			</div>
			<div class="space-y-3">
				<div>
					<label for="pn" class="mb-1 block text-sm font-medium text-slate-700">Nombre *</label>
					<input id="pn" bind:value={nombre} required class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
				</div>
				<div>
					<label for="pco" class="mb-1 block text-sm font-medium text-slate-700">Contacto (opcional)</label>
					<input id="pco" bind:value={contacto} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
				</div>
				<div>
					<label for="pt" class="mb-1 block text-sm font-medium text-slate-700">Teléfono *</label>
					<input id="pt" bind:value={telefono} inputmode="tel" required class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
				</div>
				<div>
					<label for="pe" class="mb-1 block text-sm font-medium text-slate-700">Correo electrónico (opcional)</label>
					<input id="pe" bind:value={email} type="email" inputmode="email" placeholder="correo@ejemplo.com" class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
				</div>
				{#if editandoId != null}
					<label class="flex items-center gap-2 text-sm text-slate-700">
						<input type="checkbox" bind:checked={activo} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
						Activo
					</label>
				{/if}
				{#if error}
					<p class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>
				{/if}
				<button onclick={guardar} disabled={guardando} class="w-full rounded-lg bg-indigo-600 py-2.5 font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
					{guardando ? 'Guardando…' : editandoId != null ? 'Guardar cambios' : 'Guardar proveedor'}
				</button>
			</div>
		</div>
	</div>
{/if}
