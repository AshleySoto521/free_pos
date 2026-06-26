<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { session } from '$lib/stores/session';

	const items = [
		{ titulo: 'Categorías', desc: 'Agrupar productos', emoji: '🏷️', href: '/categorias' },
		{ titulo: 'Unidades de medida', desc: 'Pieza, kilo, litro…', emoji: '⚖️', href: '/unidades' },
		{ titulo: 'Métodos de pago', desc: 'Efectivo, tarjeta…', emoji: '💳', href: '/metodos-pago' },
		{ titulo: 'Monedas', desc: 'Divisas y denominaciones', emoji: '🪙', href: '/monedas' },
		{ titulo: 'Datos del negocio', desc: 'Para el ticket', emoji: '🏪', href: '/configuracion' },
		{ titulo: 'Importar / Exportar', desc: 'Catálogos en XLSX/CSV', emoji: '📤', href: '/datos' }
	] as const;

	onMount(() => {
		if ($session?.rol !== 'Administrador') goto(resolve('/'));
	});
</script>

<div class="min-h-screen bg-slate-100">
	<header class="flex items-center gap-3 border-b border-slate-200 bg-white px-5 py-3">
		<button onclick={() => goto(resolve('/'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
		<h1 class="text-lg font-semibold text-slate-800">Catálogos</h1>
	</header>

	<main class="mx-auto max-w-5xl p-6">
		<div class="grid grid-cols-2 gap-4 sm:grid-cols-4">
			{#each items as m (m.href)}
				<button
					onclick={() => goto(resolve(m.href))}
					class="flex flex-col items-start gap-2 rounded-xl border border-slate-200 bg-white p-5 text-left shadow-sm transition hover:border-indigo-300 hover:shadow"
				>
					<span class="text-3xl">{m.emoji}</span>
					<span class="font-semibold text-slate-800">{m.titulo}</span>
					<span class="text-xs text-slate-500">{m.desc}</span>
				</button>
			{/each}
		</div>
	</main>
</div>
