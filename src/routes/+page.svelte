<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api';
	import { session, licencia, logout } from '$lib/stores/session';
	import { giro, cargarAjustes } from '$lib/stores/ajustes';

	// Primer arranque: si el admin no ha completado la configuración inicial,
	// lo llevamos al asistente de bienvenida.
	onMount(async () => {
		cargarAjustes();
		if ($session?.rol !== 'Administrador') return;
		try {
			const cfg = await api.listarConfig();
			const completo = cfg.some((c) => c.clave === 'onboarding_completo' && c.valor === '1');
			if (!completo) goto(resolve('/bienvenida'));
		} catch {
			// si falla la lectura, no bloqueamos el inicio
		}
	});

	// Módulos del POS (se irán habilitando conforme construyamos las pantallas).
	const modulos = [
		{ titulo: 'Vender', desc: 'Cobrar y generar tickets', emoji: '🛒', href: '/vender', soloAdmin: false },
		{ titulo: 'Corte de caja', desc: 'Abrir y cerrar turno', emoji: '💵', href: '/corte', soloAdmin: false },
		{ titulo: 'Inventario', desc: 'Productos y existencias', emoji: '📦', href: '/inventario', soloAdmin: true },
		{ titulo: 'Caducidades', desc: 'Lotes por vencer', emoji: '🗓️', href: '/caducidades', soloAdmin: true, soloFarmacia: true },
		{ titulo: 'Ventas', desc: 'Historial y cierre', emoji: '📊', href: '/ventas', soloAdmin: true },
		{ titulo: 'Compras', desc: 'Reabastecer inventario', emoji: '🚚', href: '/compras', soloAdmin: true },
		{ titulo: 'Proveedores', desc: 'Directorio de proveedores', emoji: '🏷️', href: '/proveedores', soloAdmin: true },
		{ titulo: 'Clientes', desc: 'Fiado y abonos', emoji: '🧾', href: '/clientes', soloAdmin: false },
		{ titulo: 'Usuarios', desc: 'Cajeros y administradores', emoji: '👥', href: '/usuarios', soloAdmin: true },
		{ titulo: 'Catálogos', desc: 'Categorías, unidades, pagos', emoji: '⚙️', href: '/catalogos', soloAdmin: true },
		{ titulo: 'Bitácora', desc: 'Auditoría de acciones', emoji: '📋', href: '/bitacora', soloAdmin: true },
		{ titulo: 'Reportes', desc: 'Ventas, inventario, cortes', emoji: '📈', href: '/reportes', soloAdmin: true }
	] as const;

	const esFarmacia = $derived(/farmac/i.test($giro));
	const visibles = $derived(
		modulos.filter(
			(m) =>
				(!m.soloAdmin || $session?.rol === 'Administrador') &&
				(!('soloFarmacia' in m) || esFarmacia)
		)
	);
</script>

<div class="min-h-screen bg-slate-100">
	<header class="flex items-center justify-between border-b border-slate-200 bg-white px-6 py-3 shadow-sm">
		<div class="flex items-center gap-3">
			<img src="/logo_pos.png" alt="Free POS" class="h-9 w-9 rounded-lg object-contain" />
			<span class="text-lg font-semibold text-slate-800">Free POS</span>
		</div>
		<div class="flex items-center gap-4">
			<div class="text-right">
				<p class="text-sm font-medium text-slate-700">{$session?.nombre}</p>
				<p class="text-xs text-slate-400">{$session?.rol}</p>
			</div>
			<button
				onclick={() => goto(resolve('/ayuda'))}
				class="rounded-lg border border-slate-300 px-3 py-1.5 text-sm font-medium text-slate-600 transition hover:bg-slate-50"
			>
				📖 Guía
			</button>
			<button
				onclick={logout}
				class="rounded-lg border border-slate-300 px-3 py-1.5 text-sm font-medium text-slate-600 transition hover:bg-slate-50"
			>
				Salir
			</button>
		</div>
	</header>

	<main class="mx-auto max-w-7xl p-6">
		{#if $licencia?.estado === 'Prueba'}
			<div class="mb-5 flex items-center justify-between gap-3 rounded-lg bg-indigo-50 px-4 py-3 text-sm text-indigo-800">
				<span>
					🎁 Versión de prueba — te quedan <strong>{$licencia.diasRestantes}</strong>
					día{$licencia.diasRestantes === 1 ? '' : 's'} gratis.
				</span>
				<button
					onclick={() => goto(resolve('/licencia'))}
					class="shrink-0 rounded-lg bg-indigo-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-indigo-700"
				>
					Activar licencia
				</button>
			</div>
		{:else if $licencia?.diasRestantes != null && $licencia.diasRestantes <= 15}
			<div class="mb-5 flex items-center justify-between gap-3 rounded-lg bg-amber-50 px-4 py-3 text-sm text-amber-800">
				<span>Tu licencia vence en {$licencia.diasRestantes} día{$licencia.diasRestantes === 1 ? '' : 's'}.</span>
				<button
					onclick={() => goto(resolve('/licencia'))}
					class="shrink-0 rounded-lg bg-amber-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-amber-700"
				>
					Renovar
				</button>
			</div>
		{/if}

		<h1 class="mb-1 text-2xl font-bold text-slate-800">Hola, {$session?.nombre?.split(' ')[0]} 👋</h1>
		<p class="mb-6 text-slate-500">¿Qué quieres hacer?</p>

		<div class="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
			{#each visibles as m (m.titulo)}
				<button
					onclick={() => m.href && goto(resolve(m.href))}
					disabled={!m.href}
					class="flex flex-col items-start gap-2 rounded-xl border border-slate-200 bg-white p-5 text-left shadow-sm transition hover:border-indigo-300 hover:shadow disabled:cursor-not-allowed disabled:opacity-60 disabled:hover:border-slate-200 disabled:hover:shadow-sm"
				>
					<span class="text-3xl">{m.emoji}</span>
					<span class="font-semibold text-slate-800">{m.titulo}</span>
					<span class="text-xs text-slate-500">{m.desc}</span>
					{#if !m.href}
						<span class="mt-1 rounded bg-slate-100 px-1.5 py-0.5 text-[10px] font-medium text-slate-500">Próximamente</span>
					{/if}
				</button>
			{/each}
		</div>
	</main>
</div>
