<script lang="ts">
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { api, type LicenseStatus } from '$lib/api';
	import { session, licencia, enTauri, setSession } from '$lib/stores/session';
	import Activar from './Activar.svelte';
	import SetupAdmin from './SetupAdmin.svelte';
	import Login from './Login.svelte';

	let { children }: { children: Snippet } = $props();

	type Fase = 'cargando' | 'sinTauri' | 'activar' | 'setup' | 'login' | 'listo';
	let fase = $state<Fase>('cargando');
	let estadoLic = $state<LicenseStatus | null>(null);

	onMount(async () => {
		if (!enTauri()) {
			fase = 'sinTauri';
			return;
		}
		await verificar();
	});

	// 1) licencia  -> 2) ¿hay usuarios?  -> setup o login
	async function verificar() {
		fase = 'cargando';
		try {
			let lic = await api.licenciaEstado();
			// Prueba anclada al servidor: pídela/refréscala cuando haga falta.
			if (lic.estado === 'RequiereTrial' || lic.estado === 'GraciaAgotada') {
				try {
					lic = await api.asegurarTrial();
				} catch {
					// Sin internet en el primer arranque: no se puede iniciar la prueba.
				}
			}
			estadoLic = lic;
			licencia.set(lic);
			if (!lic.valida) {
				fase = 'activar';
				return;
			}
			// Restaura la sesión si el backend aún la conserva (sobrevive a Ctrl+R:
			// el proceso de Rust no se reinicia al recargar la ventana).
			const activa = await api.sesionActual();
			if (activa) {
				setSession(activa);
				fase = 'listo';
				return;
			}
			await pasoUsuarios();
		} catch {
			// Si el backend falla, lo más útil es mandar a activar.
			fase = 'activar';
		}
	}

	async function pasoUsuarios() {
		const usuarios = await api.listarUsuarios();
		fase = usuarios.length === 0 ? 'setup' : 'login';
	}

	function onActivada(lic: LicenseStatus) {
		estadoLic = lic;
		licencia.set(lic);
		pasoUsuarios();
	}
</script>

{#if $session}
	{@render children()}
{:else}
	<div class="flex min-h-screen items-center justify-center bg-slate-100 p-6">
		{#if fase === 'cargando'}
			<div class="flex flex-col items-center gap-3 text-slate-500">
				<div class="h-8 w-8 animate-spin rounded-full border-4 border-slate-300 border-t-indigo-600"></div>
				<p class="text-sm">Cargando…</p>
			</div>
		{:else if fase === 'sinTauri'}
			<div class="max-w-md rounded-xl bg-white p-6 text-center shadow-sm">
				<h1 class="text-lg font-semibold text-slate-800">Abre la app desde Tauri</h1>
				<p class="mt-2 text-sm text-slate-500">
					Esta es una aplicación de escritorio. Ejecuta <code class="rounded bg-slate-100 px-1.5 py-0.5 font-mono">pnpm tauri dev</code>
					para usarla; en el navegador no hay acceso a la base de datos.
				</p>
			</div>
		{:else}
			<div class="rounded-2xl bg-white p-8 shadow-sm">
				{#if fase === 'activar'}
					<Activar estado={estadoLic} {onActivada} />
				{:else if fase === 'setup'}
					<SetupAdmin onListo={() => (fase = 'login')} />
				{:else}
					<Login />
				{/if}
			</div>
		{/if}
	</div>
{/if}
