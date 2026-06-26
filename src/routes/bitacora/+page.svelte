<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, type LogBitacora } from '$lib/api';
	import { session } from '$lib/stores/session';
	import { fechaHora } from '$lib/format';

	function hoyStr() {
		const d = new Date();
		return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
	}

	let desde = $state(hoyStr());
	let hasta = $state(hoyStr());
	let entradas = $state<LogBitacora[]>([]);
	let cargando = $state(true);

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
			entradas = await api.listarBitacora(desde, hasta);
		} finally {
			cargando = false;
		}
	}

	const fmt = (f: string) => fechaHora(f);

	function colorAccion(a: string): string {
		if (a === 'Cancelación' || a === 'Acceso fallido') return 'bg-red-100 text-red-700';
		if (a === 'Venta' || a === 'Abono' || a === 'Cierre') return 'bg-green-100 text-green-700';
		if (a === 'Inicio de sesión') return 'bg-green-100 text-green-700';
		if (a === 'Alta') return 'bg-indigo-100 text-indigo-700';
		if (a === 'Cierre de sesión') return 'bg-slate-200 text-slate-600';
		return 'bg-slate-100 text-slate-600';
	}
</script>

<div class="min-h-screen bg-slate-100">
	<header class="flex items-center gap-3 border-b border-slate-200 bg-white px-5 py-3">
		<button onclick={() => goto(resolve('/'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
		<h1 class="text-lg font-semibold text-slate-800">Bitácora</h1>
	</header>

	<main class="mx-auto max-w-6xl p-6">
		<div class="mb-4 flex flex-wrap items-end gap-3">
			<div>
				<label for="d" class="mb-1 block text-xs font-medium text-slate-500">Desde</label>
				<input id="d" type="date" bind:value={desde} class="rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
			</div>
			<div>
				<label for="h" class="mb-1 block text-xs font-medium text-slate-500">Hasta</label>
				<input id="h" type="date" bind:value={hasta} class="rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
			</div>
			<button onclick={cargar} class="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700">Aplicar</button>
		</div>

		{#if cargando}
			<p class="py-10 text-center text-slate-400">Cargando…</p>
		{:else if entradas.length === 0}
			<p class="rounded-xl border border-dashed border-slate-300 bg-white py-12 text-center text-slate-500">
				Sin movimientos en este rango.
			</p>
		{:else}
			<div class="overflow-x-auto rounded-xl border border-slate-200 bg-white">
				<table class="w-full text-sm">
					<thead class="bg-slate-50 text-left text-xs uppercase tracking-wide text-slate-500">
						<tr>
							<th class="px-4 py-2.5 font-medium">Fecha</th>
							<th class="px-4 py-2.5 font-medium">Usuario</th>
							<th class="px-4 py-2.5 font-medium">Acción</th>
							<th class="px-4 py-2.5 font-medium">Entidad</th>
							<th class="px-4 py-2.5 font-medium">Detalle</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-slate-100">
						{#each entradas as e (e.idBitacora)}
							<tr class="hover:bg-slate-50">
								<td class="px-4 py-2.5 whitespace-nowrap text-slate-500">{fmt(e.fecha)}</td>
								<td class="px-4 py-2.5 font-medium text-slate-700">{e.usuario ?? '—'}</td>
								<td class="px-4 py-2.5">
									<span class="rounded-full px-2 py-0.5 text-xs font-medium {colorAccion(e.accion)}">{e.accion}</span>
								</td>
								<td class="px-4 py-2.5 text-slate-600">{e.entidad}</td>
								<td class="px-4 py-2.5 text-slate-500">{e.detalle ?? '—'}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
			<p class="mt-2 text-xs text-slate-400">Se muestran hasta 500 registros del rango.</p>
		{/if}
	</main>
</div>
