<script lang="ts">
	import { onMount } from 'svelte';
	import { api, type Moneda, type Denominacion } from '$lib/api';

	// Cuenta billetes/monedas por denominación y reporta el total al padre.
	let { onTotal }: { onTotal: (n: number) => void } = $props();

	let monedas = $state<Moneda[]>([]);
	let idMoneda = $state<number | null>(null);
	let denominaciones = $state<Denominacion[]>([]);
	let cant = $state<Record<number, number | null>>({});

	const monedaSel = $derived(monedas.find((m) => m.idMoneda === idMoneda) ?? null);
	const simbolo = $derived(monedaSel?.simbolo ?? '$');
	const total = $derived(
		denominaciones.reduce((s, d) => s + d.valor * (cant[d.idDenominacion] || 0), 0)
	);

	$effect(() => {
		onTotal(total);
	});

	const fmt = (v: number) =>
		simbolo + v.toLocaleString('es-MX', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

	onMount(async () => {
		monedas = await api.listarMonedas();
		const principal = monedas.find((m) => m.esPrincipal) ?? monedas[0];
		if (principal) {
			idMoneda = principal.idMoneda;
			await cargar();
		}
	});

	async function cargar() {
		if (idMoneda == null) return;
		denominaciones = await api.listarDenominaciones(idMoneda);
		cant = {};
	}
</script>

<div class="rounded-lg border border-slate-200 bg-slate-50 p-3">
	{#if monedas.length > 1}
		<select bind:value={idMoneda} onchange={cargar} class="mb-2 w-full rounded-lg border-slate-300 text-sm shadow-sm focus:border-indigo-500 focus:ring-indigo-500">
			{#each monedas as m (m.idMoneda)}
				<option value={m.idMoneda}>{m.moneda} ({m.codigo})</option>
			{/each}
		</select>
	{/if}

	{#if denominaciones.length === 0}
		<p class="py-2 text-center text-xs text-slate-400">Sin denominaciones para esta moneda.</p>
	{:else}
		<div class="space-y-1.5">
			{#each denominaciones as d (d.idDenominacion)}
				<div class="flex items-center gap-2 text-sm">
					<span class="w-20 text-right font-medium text-slate-700">{fmt(d.valor)}</span>
					<span class="text-slate-400">×</span>
					<input
						type="number"
						min="0"
						bind:value={cant[d.idDenominacion]}
						placeholder="0"
						class="w-16 rounded-lg border-slate-300 py-1 text-center text-sm shadow-sm focus:border-indigo-500 focus:ring-indigo-500"
					/>
					<span class="ml-auto w-24 text-right text-slate-600">{fmt(d.valor * (cant[d.idDenominacion] || 0))}</span>
				</div>
			{/each}
			<div class="flex justify-between border-t border-slate-200 pt-2 font-semibold text-slate-800">
				<span>Total contado</span><span>{fmt(total)}</span>
			</div>
		</div>
	{/if}
</div>
