<script lang="ts">
	import { api } from '$lib/api';
	import { setSession } from '$lib/stores/session';

	let usuario = $state('');
	let contrasena = $state('');
	let cargando = $state(false);
	let error = $state('');

	async function entrar(e: Event) {
		e.preventDefault();
		cargando = true;
		error = '';
		try {
			const u = await api.login(usuario.trim(), contrasena);
			setSession(u);
		} catch (err) {
			error = String(err);
			contrasena = '';
		} finally {
			cargando = false;
		}
	}
</script>

<form onsubmit={entrar} class="w-full max-w-sm space-y-5">
	<div class="text-center">
		<img src="/logo_pos.png" alt="AquaPOS" class="mx-auto mb-3 h-16 w-16 rounded-xl object-contain" />
		<h1 class="text-xl font-semibold text-slate-800">Iniciar sesión</h1>
		<p class="mt-1 text-sm text-slate-500">Entra con tu usuario y contraseña.</p>
	</div>

	<div>
		<label for="usuario" class="mb-1 block text-sm font-medium text-slate-700">Usuario</label>
		<input id="usuario" bind:value={usuario} required autocomplete="username" class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
	</div>

	<div>
		<label for="pass" class="mb-1 block text-sm font-medium text-slate-700">Contraseña</label>
		<input id="pass" type="password" bind:value={contrasena} required autocomplete="current-password" class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
	</div>

	{#if error}
		<p class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>
	{/if}

	<button
		type="submit"
		disabled={cargando || !usuario.trim() || !contrasena}
		class="w-full rounded-lg bg-indigo-600 px-4 py-2.5 font-medium text-white shadow-sm transition hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-50"
	>
		{cargando ? 'Entrando…' : 'Entrar'}
	</button>
</form>
