<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api';
	import { session } from '$lib/stores/session';
	import { modoVenta, giro, type ModoVenta } from '$lib/stores/ajustes';

	const campos = [
		{ clave: 'NombreTienda', label: 'Nombre del negocio' },
		{ clave: 'Direccion', label: 'Dirección' },
		{ clave: 'Telefono', label: 'Teléfono' },
		{ clave: 'PieTicket', label: 'Pie de ticket (mensaje al final)' }
	];

	const modos: { valor: ModoVenta; label: string; desc: string }[] = [
		{ valor: 'productos', label: 'Solo productos', desc: 'Manejan inventario y stock' },
		{ valor: 'servicios', label: 'Solo servicios', desc: 'Mano de obra, sin inventario' },
		{ valor: 'ambos', label: 'Ambos', desc: 'Productos y servicios' }
	];

	let valores = $state<Record<string, string>>({});
	let modoSel = $state<ModoVenta>('productos');
	let giroSel = $state('');
	let cargando = $state(true);
	let guardando = $state(false);
	let guardado = $state(false);
	let error = $state('');

	onMount(async () => {
		if ($session?.rol !== 'Administrador') {
			goto(resolve('/'));
			return;
		}
		try {
			const items = await api.listarConfig();
			const map: Record<string, string> = {};
			for (const c of campos) map[c.clave] = '';
			for (const it of items) map[it.clave] = it.valor;
			valores = map;
			const m = items.find((it) => it.clave === 'modo_venta')?.valor;
			if (m === 'productos' || m === 'servicios' || m === 'ambos') modoSel = m;
			giroSel = items.find((it) => it.clave === 'giro')?.valor ?? '';
		} finally {
			cargando = false;
		}
	});

	async function guardar() {
		guardando = true;
		guardado = false;
		error = '';
		try {
			await api.guardarConfig([
				...campos.map((c) => ({ clave: c.clave, valor: valores[c.clave] ?? '' })),
				{ clave: 'modo_venta', valor: modoSel },
				{ clave: 'giro', valor: giroSel.trim() }
			]);
			// Refleja el cambio en el resto de la app sin recargar.
			modoVenta.set(modoSel);
			giro.set(giroSel.trim());
			guardado = true;
		} catch (e) {
			error = String(e);
		} finally {
			guardando = false;
		}
	}
</script>

<div class="flex min-h-screen flex-col bg-slate-100">
	<header class="flex items-center gap-3 border-b border-slate-200 bg-white px-5 py-3">
		<button onclick={() => goto(resolve('/catalogos'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
		<h1 class="text-lg font-semibold text-slate-800">Datos del negocio</h1>
	</header>

	<main class="mx-auto flex w-full max-w-lg flex-1 flex-col justify-center p-6">
		{#if cargando}
			<p class="py-10 text-center text-slate-400">Cargando…</p>
		{:else}
			<div class="space-y-4 rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
				<!-- Giro y modo de venta -->
				<div>
					<label for="giro" class="mb-1 block text-sm font-medium text-slate-700">Giro del negocio (opcional)</label>
					<input id="giro" bind:value={giroSel} placeholder="Ej. Farmacia, Refaccionaria, Peluquería…" class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
				</div>

				<div>
					<span class="mb-1 block text-sm font-medium text-slate-700">¿Qué vendes?</span>
					<div class="grid grid-cols-3 gap-2">
						{#each modos as m (m.valor)}
							<button
								type="button"
								onclick={() => (modoSel = m.valor)}
								class="rounded-lg border px-2 py-2 text-center text-sm transition {modoSel === m.valor
									? 'border-indigo-500 bg-indigo-50 text-indigo-700'
									: 'border-slate-300 text-slate-600 hover:bg-slate-50'}"
							>
								<span class="block font-semibold">{m.label}</span>
								<span class="block text-[11px] leading-tight text-slate-400">{m.desc}</span>
							</button>
						{/each}
					</div>
				</div>

				<hr class="border-slate-100" />
				<p class="text-sm text-slate-500">Estos datos aparecerán en el ticket de venta.</p>
				{#each campos as c (c.clave)}
					<div>
						<label for={c.clave} class="mb-1 block text-sm font-medium text-slate-700">{c.label}</label>
						<input id={c.clave} bind:value={valores[c.clave]} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					</div>
				{/each}

				{#if error}<p class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>{/if}
				{#if guardado}<p class="rounded-lg bg-green-50 px-3 py-2 text-sm text-green-700">Guardado ✓</p>{/if}

				<button onclick={guardar} disabled={guardando} class="w-full rounded-lg bg-indigo-600 py-2.5 font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
					{guardando ? 'Guardando…' : 'Guardar'}
				</button>
			</div>
		{/if}
	</main>
</div>
