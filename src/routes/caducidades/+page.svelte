<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, type LoteVencimiento } from '$lib/api';
	import { session } from '$lib/stores/session';
	import { toast } from '$lib/stores/toast';

	let dias = $state(30);
	let lotes = $state<LoteVencimiento[]>([]);
	let cargando = $state(true);

	let loteABaja = $state<LoteVencimiento | null>(null);
	let dandoBaja = $state(false);
	let errorBaja = $state('');

	const filtros = [
		{ label: 'Vencidos', dias: 0 },
		{ label: '30 días', dias: 30 },
		{ label: '60 días', dias: 60 },
		{ label: '90 días', dias: 90 }
	];

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
			lotes = await api.lotesPorVencer(dias);
		} finally {
			cargando = false;
		}
	}

	function aplicar(d: number) {
		dias = d;
		cargar();
	}

	function estado(l: LoteVencimiento): { texto: string; clase: string } {
		const d = l.diasRestantes ?? 0;
		if (d < 0) return { texto: `Vencido (${-d} d)`, clase: 'bg-red-100 text-red-700' };
		if (d <= 7) return { texto: `${d} día${d === 1 ? '' : 's'}`, clase: 'bg-red-100 text-red-700' };
		if (d <= 30) return { texto: `${d} días`, clase: 'bg-amber-100 text-amber-700' };
		return { texto: `${d} días`, clase: 'bg-slate-100 text-slate-600' };
	}

	async function confirmarBaja() {
		if (!loteABaja) return;
		dandoBaja = true;
		errorBaja = '';
		try {
			await api.darBajaLote(loteABaja.idLote, 'Merma por caducidad', $session?.idUsuario ?? null);
			toast('Lote dado de baja', 'ok');
			loteABaja = null;
			await cargar();
		} catch (e) {
			errorBaja = String(e);
		} finally {
			dandoBaja = false;
		}
	}
</script>

<div class="min-h-screen bg-slate-100">
	<header class="flex items-center gap-3 border-b border-slate-200 bg-white px-5 py-3">
		<button onclick={() => goto(resolve('/'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
		<h1 class="text-lg font-semibold text-slate-800">Caducidades</h1>
	</header>

	<main class="mx-auto max-w-5xl p-6">
		<div class="mb-4 flex flex-wrap gap-2">
			{#each filtros as f (f.dias)}
				<button
					onclick={() => aplicar(f.dias)}
					class="rounded-lg border px-3 py-1.5 text-sm font-medium transition {dias === f.dias
						? 'border-indigo-500 bg-indigo-50 text-indigo-700'
						: 'border-slate-300 text-slate-600 hover:bg-slate-50'}"
				>
					{f.label}
				</button>
			{/each}
		</div>

		{#if cargando}
			<p class="py-10 text-center text-slate-400">Cargando…</p>
		{:else if lotes.length === 0}
			<div class="rounded-xl border border-dashed border-slate-300 bg-white py-12 text-center text-slate-500">
				🎉 No hay lotes por vencer en este rango.
			</div>
		{:else}
			<div class="overflow-x-auto rounded-xl border border-slate-200 bg-white">
				<table class="w-full text-sm">
					<thead class="bg-slate-50 text-left text-xs uppercase tracking-wide text-slate-500">
						<tr>
							<th class="px-4 py-2.5 font-medium">Producto</th>
							<th class="px-4 py-2.5 font-medium">Lote</th>
							<th class="px-4 py-2.5 font-medium">Caducidad</th>
							<th class="px-4 py-2.5 text-right font-medium">Cantidad</th>
							<th class="px-4 py-2.5 font-medium">Estado</th>
							<th class="px-4 py-2.5 text-right font-medium">Acciones</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-slate-100">
						{#each lotes as l (l.idLote)}
							<tr class="hover:bg-slate-50">
								<td class="px-4 py-2.5 font-medium text-slate-800">{l.producto}</td>
								<td class="px-4 py-2.5 text-slate-600">{l.lote ?? '—'}</td>
								<td class="px-4 py-2.5 text-slate-600">{l.caducidad ?? '—'}</td>
								<td class="px-4 py-2.5 text-right text-slate-700">{l.cantidad}</td>
								<td class="px-4 py-2.5">
									<span class="rounded-full px-2 py-0.5 text-xs font-medium {estado(l).clase}">{estado(l).texto}</span>
								</td>
								<td class="px-4 py-2.5 text-right">
									<button onclick={() => { loteABaja = l; errorBaja = ''; }} class="rounded-lg border border-red-200 bg-red-50 px-2.5 py-1 text-xs font-medium text-red-700 hover:bg-red-100">
										Dar de baja
									</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</main>
</div>

<!-- Confirmar baja de lote -->
{#if loteABaja}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="w-full max-w-sm rounded-2xl bg-white p-6 shadow-xl">
			<h2 class="mb-2 text-lg font-semibold text-slate-800">Dar de baja lote</h2>
			<p class="mb-4 text-sm text-slate-600">
				Se dará de baja por <strong>merma</strong> el lote
				{#if loteABaja.lote}<strong>{loteABaja.lote}</strong>{/if}
				de <strong>{loteABaja.producto}</strong>
				(<strong>{loteABaja.cantidad}</strong> uds, caduca {loteABaja.caducidad}). Se descuenta del
				inventario y queda en la bitácora. Esto no se puede deshacer.
			</p>
			{#if errorBaja}
				<p class="mb-2 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{errorBaja}</p>
			{/if}
			<div class="flex gap-2">
				<button onclick={() => (loteABaja = null)} class="flex-1 rounded-lg border border-slate-300 py-2 text-sm text-slate-600 hover:bg-slate-50">No</button>
				<button onclick={confirmarBaja} disabled={dandoBaja} class="flex-1 rounded-lg bg-red-600 py-2 text-sm font-semibold text-white hover:bg-red-700 disabled:opacity-50">
					{dandoBaja ? 'Procesando…' : 'Sí, dar de baja'}
				</button>
			</div>
		</div>
	</div>
{/if}
