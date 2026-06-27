<script lang="ts">
	import { api } from '$lib/api';
	import { setSession } from '$lib/stores/session';

	let { onListo }: { onListo: () => void } = $props();

	let nombre = $state('');
	let usuario = $state('');
	let contrasena = $state('');
	let confirmar = $state('');
	let cargando = $state(false);
	let error = $state('');

	async function crear(e: Event) {
		e.preventDefault();
		if (contrasena !== confirmar) {
			error = 'Las contraseñas no coinciden.';
			return;
		}
		if (contrasena.length < 4) {
			error = 'La contraseña debe tener al menos 4 caracteres.';
			return;
		}
		cargando = true;
		error = '';
		try {
			await api.crearUsuario({
				nombre: nombre.trim(),
				usuario: usuario.trim(),
				contrasena,
				rol: 'Administrador'
			});
			// Auto-login del admin recién creado.
			const u = await api.login(usuario.trim(), contrasena);
			onListo(); // avisa al Gate para que el logout regrese a Login, no a Setup
			setSession(u);
		} catch (err) {
			error = String(err);
		} finally {
			cargando = false;
		}
	}
</script>

<form onsubmit={crear} class="w-full max-w-sm space-y-4">
	<div class="text-center">
		<img src="/logo_pos.png" alt="AquaPOS" class="mx-auto mb-3 h-16 w-16 rounded-xl object-contain" />
		<h1 class="text-xl font-semibold text-slate-800">Configuración inicial</h1>
		<p class="mt-1 text-sm text-slate-500">Crea el usuario administrador para empezar.</p>
	</div>

	<div>
		<label for="nombre" class="mb-1 block text-sm font-medium text-slate-700">Nombre completo</label>
		<input id="nombre" bind:value={nombre} required class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
	</div>

	<div>
		<label for="usuario" class="mb-1 block text-sm font-medium text-slate-700">Usuario</label>
		<input id="usuario" bind:value={usuario} required autocomplete="username" class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
	</div>

	<div class="grid grid-cols-2 gap-3">
		<div>
			<label for="pass" class="mb-1 block text-sm font-medium text-slate-700">Contraseña</label>
			<input id="pass" type="password" bind:value={contrasena} required autocomplete="new-password" class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
		</div>
		<div>
			<label for="pass2" class="mb-1 block text-sm font-medium text-slate-700">Confirmar</label>
			<input id="pass2" type="password" bind:value={confirmar} required autocomplete="new-password" class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
		</div>
	</div>

	{#if error}
		<p class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>
	{/if}

	<button
		type="submit"
		disabled={cargando}
		class="w-full rounded-lg bg-indigo-600 px-4 py-2.5 font-medium text-white shadow-sm transition hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-50"
	>
		{cargando ? 'Creando…' : 'Crear administrador'}
	</button>
</form>
