<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, type Usuario } from '$lib/api';
	import { session } from '$lib/stores/session';

	const esAdmin = $derived($session?.rol === 'Administrador');

	let usuarios = $state<Usuario[]>([]);
	let cargando = $state(true);

	// --- Modal usuario (crear/editar) ---
	let modal = $state(false);
	let editandoId = $state<number | null>(null);
	let nombre = $state('');
	let usuario = $state('');
	let contrasena = $state('');
	let rol = $state('Cajero');
	let activo = $state(true);
	let guardando = $state(false);
	let error = $state('');

	// --- Modal contraseña ---
	let modalPass = $state(false);
	let passId = $state<number | null>(null);
	let passNombre = $state('');
	let nuevaPass = $state('');
	let confirmarPass = $state('');
	let guardandoPass = $state(false);
	let errorPass = $state('');

	onMount(async () => {
		if (!esAdmin) {
			cargando = false;
			return;
		}
		await cargar();
	});

	async function cargar() {
		cargando = true;
		try {
			usuarios = await api.listarUsuarios();
		} finally {
			cargando = false;
		}
	}

	function abrirNuevo() {
		editandoId = null;
		nombre = '';
		usuario = '';
		contrasena = '';
		rol = 'Cajero';
		activo = true;
		error = '';
		modal = true;
	}

	function abrirEditar(u: Usuario) {
		editandoId = u.idUsuario;
		nombre = u.nombre;
		usuario = u.usuario;
		rol = u.rol;
		activo = u.activo;
		error = '';
		modal = true;
	}

	async function guardar() {
		if (!nombre.trim() || !usuario.trim() || (editandoId == null && !contrasena)) {
			error = 'Completa todos los campos.';
			return;
		}
		if (editandoId == null && contrasena.length < 4) {
			error = 'La contraseña debe tener al menos 4 caracteres.';
			return;
		}
		guardando = true;
		error = '';
		try {
			if (editandoId != null) {
				await api.actualizarUsuario(editandoId, {
					nombre: nombre.trim(),
					usuario: usuario.trim(),
					rol,
					activo
				});
			} else {
				await api.crearUsuario({ nombre: nombre.trim(), usuario: usuario.trim(), contrasena, rol });
			}
			await cargar();
			modal = false;
		} catch (e) {
			error = String(e);
		} finally {
			guardando = false;
		}
	}

	function abrirPass(u: Usuario) {
		passId = u.idUsuario;
		passNombre = u.nombre;
		nuevaPass = '';
		confirmarPass = '';
		errorPass = '';
		modalPass = true;
	}

	async function guardarPass() {
		if (passId == null) return;
		if (nuevaPass.length < 4) {
			errorPass = 'La contraseña debe tener al menos 4 caracteres.';
			return;
		}
		if (nuevaPass !== confirmarPass) {
			errorPass = 'Las contraseñas no coinciden.';
			return;
		}
		guardandoPass = true;
		errorPass = '';
		try {
			await api.cambiarContrasena(passId, nuevaPass);
			modalPass = false;
		} catch (e) {
			errorPass = String(e);
		} finally {
			guardandoPass = false;
		}
	}
</script>

<div class="min-h-screen bg-slate-100">
	<header class="flex items-center justify-between border-b border-slate-200 bg-white px-5 py-3">
		<div class="flex items-center gap-3">
			<button onclick={() => goto(resolve('/'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
			<h1 class="text-lg font-semibold text-slate-800">Usuarios</h1>
		</div>
		{#if esAdmin}
			<button onclick={abrirNuevo} class="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700">
				+ Nuevo usuario
			</button>
		{/if}
	</header>

	<main class="mx-auto max-w-5xl p-6">
		{#if !esAdmin}
			<div class="rounded-xl border border-amber-200 bg-amber-50 p-6 text-center text-amber-800">
				Solo los administradores pueden gestionar usuarios.
			</div>
		{:else if cargando}
			<p class="py-10 text-center text-slate-400">Cargando…</p>
		{:else}
			<div class="overflow-x-auto rounded-xl border border-slate-200 bg-white">
				<table class="w-full text-sm">
					<thead class="bg-slate-50 text-left text-xs uppercase tracking-wide text-slate-500">
						<tr>
							<th class="px-4 py-2.5 font-medium">Nombre</th>
							<th class="px-4 py-2.5 font-medium">Usuario</th>
							<th class="px-4 py-2.5 font-medium">Rol</th>
							<th class="px-4 py-2.5 font-medium">Estado</th>
							<th class="px-4 py-2.5 text-right font-medium">Acciones</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-slate-100">
						{#each usuarios as u (u.idUsuario)}
							<tr class="hover:bg-slate-50 {u.activo ? '' : 'opacity-50'}">
								<td class="px-4 py-2.5 font-medium text-slate-800">{u.nombre}</td>
								<td class="px-4 py-2.5 text-slate-600">{u.usuario}</td>
								<td class="px-4 py-2.5">
									<span class="rounded-full px-2 py-0.5 text-xs font-medium {u.rol === 'Administrador' ? 'bg-indigo-100 text-indigo-700' : 'bg-slate-100 text-slate-600'}">{u.rol}</span>
								</td>
								<td class="px-4 py-2.5">
									{#if u.activo}
										<span class="text-xs text-green-600">Activo</span>
									{:else}
										<span class="text-xs text-slate-400">Inactivo</span>
									{/if}
								</td>
								<td class="px-4 py-2.5 text-right whitespace-nowrap">
									<button onclick={() => abrirPass(u)} class="rounded-lg border border-slate-200 px-2.5 py-1 text-xs font-medium text-slate-600 hover:bg-slate-50">Contraseña</button>
									<button onclick={() => abrirEditar(u)} class="rounded-lg border border-indigo-200 bg-indigo-50 px-2.5 py-1 text-xs font-medium text-indigo-700 hover:bg-indigo-100">Editar</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</main>
</div>

<!-- Modal crear/editar usuario -->
{#if modal}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="w-full max-w-sm rounded-2xl bg-white p-6 shadow-xl">
			<div class="mb-4 flex items-center justify-between">
				<h2 class="text-lg font-semibold text-slate-800">{editandoId != null ? 'Editar usuario' : 'Nuevo usuario'}</h2>
				<button onclick={() => (modal = false)} class="text-slate-400 hover:text-slate-600" aria-label="Cerrar">✕</button>
			</div>

			<div class="space-y-3">
				<div>
					<label for="n" class="mb-1 block text-sm font-medium text-slate-700">Nombre completo</label>
					<input id="n" bind:value={nombre} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
				</div>
				<div>
					<label for="u" class="mb-1 block text-sm font-medium text-slate-700">Usuario</label>
					<input id="u" bind:value={usuario} autocomplete="off" class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
				</div>
				{#if editandoId == null}
					<div>
						<label for="p" class="mb-1 block text-sm font-medium text-slate-700">Contraseña</label>
						<input id="p" type="password" bind:value={contrasena} autocomplete="new-password" class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					</div>
				{/if}
				<div>
					<label for="r" class="mb-1 block text-sm font-medium text-slate-700">Rol</label>
					<select id="r" bind:value={rol} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500">
						<option value="Cajero">Cajero</option>
						<option value="Administrador">Administrador</option>
					</select>
				</div>
				{#if editandoId != null}
					<label class="flex items-center gap-2 text-sm text-slate-700">
						<input type="checkbox" bind:checked={activo} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
						Activo (puede iniciar sesión)
					</label>
				{/if}

				{#if error}
					<p class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>
				{/if}

				<button onclick={guardar} disabled={guardando} class="w-full rounded-lg bg-indigo-600 py-2.5 font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
					{guardando ? 'Guardando…' : editandoId != null ? 'Guardar cambios' : 'Crear usuario'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Modal cambiar contraseña -->
{#if modalPass}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="w-full max-w-xs rounded-2xl bg-white p-6 shadow-xl">
			<h2 class="mb-1 text-base font-semibold text-slate-800">Cambiar contraseña</h2>
			<p class="mb-3 text-sm text-slate-500">{passNombre}</p>
			<label for="np" class="mb-1 block text-sm font-medium text-slate-700">Nueva contraseña</label>
			<input id="np" type="password" bind:value={nuevaPass} autocomplete="new-password" class="mb-2 w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
			<label for="cp" class="mb-1 block text-sm font-medium text-slate-700">Confirmar</label>
			<input id="cp" type="password" bind:value={confirmarPass} autocomplete="new-password" class="mb-3 w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
			{#if errorPass}
				<p class="mb-2 text-sm text-red-700">{errorPass}</p>
			{/if}
			<div class="flex gap-2">
				<button onclick={() => (modalPass = false)} class="flex-1 rounded-lg border border-slate-300 py-2 text-sm text-slate-600 hover:bg-slate-50">Cancelar</button>
				<button onclick={guardarPass} disabled={guardandoPass} class="flex-1 rounded-lg bg-indigo-600 py-2 text-sm font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
					{guardandoPass ? 'Guardando…' : 'Cambiar'}
				</button>
			</div>
		</div>
	</div>
{/if}
