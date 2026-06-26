<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api, type Cliente, type MovimientoCliente } from '$lib/api';
	import { session } from '$lib/stores/session';
	import { pesos, fechaHora } from '$lib/format';

	const esAdmin = $derived($session?.rol === 'Administrador');

	let clientes = $state<Cliente[]>([]);
	let cargando = $state(true);
	let busqueda = $state('');
	let verInactivos = $state(false);

	const filtrados = $derived(
		busqueda.trim()
			? clientes.filter((c) =>
					(c.nombre + ' ' + (c.telefono ?? '')).toLowerCase().includes(busqueda.toLowerCase())
				)
			: clientes
	);
	const totalFiado = $derived(clientes.reduce((s, c) => s + c.saldoFiado, 0));

	// Modal cliente (crear/editar)
	let modalCli = $state(false);
	let editandoId = $state<number | null>(null);
	let nombre = $state('');
	let telefono = $state('');
	let activo = $state(true);
	let errorCli = $state('');
	let guardando = $state(false);

	// Modal abono
	let modalAbono = $state(false);
	let clienteSel = $state<Cliente | null>(null);
	let montoAbono = $state<number | null>(null);
	let errorAbono = $state('');
	let abonando = $state(false);

	// Modal estado de cuenta
	let modalCuenta = $state(false);
	let cuentaCliente = $state<Cliente | null>(null);
	let movimientos = $state<MovimientoCliente[]>([]);
	let cargandoCuenta = $state(false);

	async function abrirCuenta(c: Cliente) {
		cuentaCliente = c;
		movimientos = [];
		cargandoCuenta = true;
		modalCuenta = true;
		try {
			movimientos = await api.movimientosCliente(c.idCliente);
		} finally {
			cargandoCuenta = false;
		}
	}

	const fmtFecha = (f: string) => fechaHora(f);

	onMount(cargar); // Clientes lo pueden ver cajero y admin

	async function cargar() {
		cargando = true;
		try {
			clientes = await api.listarClientes(verInactivos);
		} finally {
			cargando = false;
		}
	}

	function abrirNuevo() {
		editandoId = null;
		nombre = '';
		telefono = '';
		activo = true;
		errorCli = '';
		modalCli = true;
	}

	function abrirEditar(c: Cliente) {
		editandoId = c.idCliente;
		nombre = c.nombre;
		telefono = c.telefono ?? '';
		activo = c.activo;
		errorCli = '';
		modalCli = true;
	}

	async function guardarCliente() {
		if (!nombre.trim()) {
			errorCli = 'El nombre es obligatorio.';
			return;
		}
		guardando = true;
		errorCli = '';
		try {
			if (editandoId != null) {
				await api.actualizarCliente(editandoId, {
					nombre: nombre.trim(),
					telefono: telefono.trim() || null,
					activo
				});
			} else {
				await api.crearCliente({ nombre: nombre.trim(), telefono: telefono.trim() || null });
			}
			await cargar();
			modalCli = false;
		} catch (e) {
			errorCli = String(e);
		} finally {
			guardando = false;
		}
	}

	function abrirAbono(c: Cliente) {
		clienteSel = c;
		montoAbono = null;
		errorAbono = '';
		modalAbono = true;
	}

	async function guardarAbono() {
		if (!clienteSel || !montoAbono || montoAbono <= 0) {
			errorAbono = 'Escribe un monto mayor a cero.';
			return;
		}
		if (montoAbono > clienteSel.saldoFiado) {
			errorAbono = `El abono no puede ser mayor a la deuda (${pesos(clienteSel.saldoFiado)}).`;
			return;
		}
		abonando = true;
		errorAbono = '';
		try {
			await api.registrarAbono(clienteSel.idCliente, montoAbono, $session?.idUsuario ?? null, null);
			await cargar();
			modalAbono = false;
		} catch (e) {
			errorAbono = String(e);
		} finally {
			abonando = false;
		}
	}
</script>

<div class="min-h-screen bg-slate-100">
	<header class="flex items-center justify-between border-b border-slate-200 bg-white px-5 py-3">
		<div class="flex items-center gap-3">
			<button onclick={() => goto(resolve('/'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
			<h1 class="text-lg font-semibold text-slate-800">Clientes</h1>
		</div>
		<button onclick={abrirNuevo} class="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700">
			+ Nuevo cliente
		</button>
	</header>

	<main class="mx-auto max-w-5xl p-6">
		<div class="mb-4 flex flex-wrap items-center gap-3">
			<input
				bind:value={busqueda}
				placeholder="Buscar cliente…"
				class="w-full max-w-xs rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500"
			/>
			<label class="flex items-center gap-2 text-sm text-slate-600">
				<input type="checkbox" bind:checked={verInactivos} onchange={cargar} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
				Ver inactivos
			</label>
			{#if totalFiado > 0}
				<span class="ml-auto rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">
					Fiado total: <strong>{pesos(totalFiado)}</strong>
				</span>
			{/if}
		</div>

		{#if cargando}
			<p class="py-10 text-center text-slate-400">Cargando…</p>
		{:else if clientes.length === 0}
			<div class="rounded-xl border border-dashed border-slate-300 bg-white py-12 text-center">
				<p class="text-slate-500">Aún no hay clientes.</p>
				<button onclick={abrirNuevo} class="mt-3 rounded-lg bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700">
					Agregar el primero
				</button>
			</div>
		{:else}
			<div class="overflow-x-auto rounded-xl border border-slate-200 bg-white">
				<table class="w-full text-sm">
					<thead class="bg-slate-50 text-left text-xs uppercase tracking-wide text-slate-500">
						<tr>
							<th class="px-4 py-2.5 font-medium">Cliente</th>
							<th class="px-4 py-2.5 font-medium">Teléfono</th>
							<th class="px-4 py-2.5 text-right font-medium">Debe</th>
							<th class="px-4 py-2.5 text-right font-medium">Acciones</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-slate-100">
						{#each filtrados as c (c.idCliente)}
							<tr class="hover:bg-slate-50 {c.activo ? '' : 'opacity-50'}">
								<td class="px-4 py-2.5 font-medium text-slate-800">
									{c.nombre}
									{#if !c.activo}<span class="ml-1 rounded bg-slate-200 px-1.5 py-0.5 text-[10px] font-medium text-slate-500">inactivo</span>{/if}
								</td>
								<td class="px-4 py-2.5 text-slate-600">{c.telefono ?? '—'}</td>
								<td class="px-4 py-2.5 text-right font-medium {c.saldoFiado > 0 ? 'text-red-600' : 'text-slate-400'}">{pesos(c.saldoFiado)}</td>
								<td class="px-4 py-2.5 text-right whitespace-nowrap">
									{#if c.saldoFiado > 0}
										<button onclick={() => abrirAbono(c)} class="rounded-lg border border-green-200 bg-green-50 px-2.5 py-1 text-xs font-medium text-green-700 hover:bg-green-100">Abonar</button>
									{/if}
									<button onclick={() => abrirCuenta(c)} class="rounded-lg border border-slate-200 px-2.5 py-1 text-xs font-medium text-slate-600 hover:bg-slate-50">Cuenta</button>
									<button onclick={() => abrirEditar(c)} class="rounded-lg border border-indigo-200 bg-indigo-50 px-2.5 py-1 text-xs font-medium text-indigo-700 hover:bg-indigo-100">Editar</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</main>
</div>

<!-- Modal crear/editar cliente -->
{#if modalCli}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="w-full max-w-sm rounded-2xl bg-white p-6 shadow-xl">
			<div class="mb-4 flex items-center justify-between">
				<h2 class="text-lg font-semibold text-slate-800">{editandoId != null ? 'Editar cliente' : 'Nuevo cliente'}</h2>
				<button onclick={() => (modalCli = false)} class="text-slate-400 hover:text-slate-600" aria-label="Cerrar">✕</button>
			</div>
			<div class="space-y-3">
				<div>
					<label for="cn" class="mb-1 block text-sm font-medium text-slate-700">Nombre</label>
					<input id="cn" bind:value={nombre} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
				</div>
				<div>
					<label for="ct" class="mb-1 block text-sm font-medium text-slate-700">Teléfono (opcional)</label>
					<input id="ct" bind:value={telefono} inputmode="tel" class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
				</div>
				{#if editandoId != null && esAdmin}
					<label class="flex items-center gap-2 text-sm text-slate-700">
						<input type="checkbox" bind:checked={activo} class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500" />
						Activo
					</label>
				{/if}
				{#if errorCli}
					<p class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{errorCli}</p>
				{/if}
				<button onclick={guardarCliente} disabled={guardando} class="w-full rounded-lg bg-indigo-600 py-2.5 font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
					{guardando ? 'Guardando…' : editandoId != null ? 'Guardar cambios' : 'Guardar cliente'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Modal abono -->
{#if modalAbono && clienteSel}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="w-full max-w-xs rounded-2xl bg-white p-6 shadow-xl">
			<h2 class="mb-1 text-base font-semibold text-slate-800">Abono de {clienteSel.nombre}</h2>
			<div class="mb-3 flex justify-between rounded-lg bg-red-50 px-3 py-2 text-sm">
				<span class="text-red-600">Debe</span><span class="font-semibold text-red-700">{pesos(clienteSel.saldoFiado)}</span>
			</div>
			<label for="ab" class="mb-1 block text-sm font-medium text-slate-700">Monto del abono</label>
			<input id="ab" type="number" step="0.01" min="0" bind:value={montoAbono} class="mb-3 w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
			{#if errorAbono}
				<p class="mb-2 text-sm text-red-700">{errorAbono}</p>
			{/if}
			<div class="flex gap-2">
				<button onclick={() => (modalAbono = false)} class="flex-1 rounded-lg border border-slate-300 py-2 text-sm text-slate-600 hover:bg-slate-50">Cancelar</button>
				<button onclick={guardarAbono} disabled={abonando} class="flex-1 rounded-lg bg-green-600 py-2 text-sm font-semibold text-white hover:bg-green-700 disabled:opacity-50">
					{abonando ? 'Guardando…' : 'Registrar abono'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Modal estado de cuenta -->
{#if modalCuenta && cuentaCliente}
	<div class="fixed inset-0 z-10 flex items-start justify-center overflow-y-auto bg-black/40 p-4">
		<div class="flex max-h-[85vh] w-full max-w-md flex-col rounded-2xl bg-white p-6 shadow-xl">
			<div class="mb-3 flex items-start justify-between">
				<div>
					<h2 class="text-lg font-semibold text-slate-800">Estado de cuenta</h2>
					<p class="text-sm text-slate-500">{cuentaCliente.nombre}</p>
				</div>
				<button onclick={() => (modalCuenta = false)} class="text-slate-400 hover:text-slate-600" aria-label="Cerrar">✕</button>
			</div>

			<div class="mb-3 rounded-xl p-3 text-center {cuentaCliente.saldoFiado > 0 ? 'bg-red-50' : 'bg-green-50'}">
				{#if cuentaCliente.saldoFiado > 0}
					<p class="text-sm text-red-600">Debe</p>
					<p class="text-2xl font-bold text-red-700">{pesos(cuentaCliente.saldoFiado)}</p>
				{:else}
					<p class="text-sm text-green-700">Sin deuda · al corriente ✓</p>
				{/if}
			</div>

			<div class="min-h-0 flex-1 overflow-y-auto">
				{#if cargandoCuenta}
					<p class="py-6 text-center text-sm text-slate-400">Cargando…</p>
				{:else if movimientos.length === 0}
					<p class="py-6 text-center text-sm text-slate-400">Sin movimientos de fiado.</p>
				{:else}
					<ul class="divide-y divide-slate-100">
						{#each movimientos as m, i (i)}
							<li class="flex items-center justify-between py-2 text-sm">
								<div>
									<span class="font-medium {m.tipo === 'Abono' ? 'text-green-600' : 'text-slate-700'}">
										{m.tipo === 'Abono' ? 'Abono' : 'Compra a fiado'}
									</span>
									{#if m.referencia}<span class="text-slate-400"> · {m.referencia}</span>{/if}
									<span class="ml-1 text-xs text-slate-400">{fmtFecha(m.fecha)}</span>
								</div>
								<span class="font-medium {m.tipo === 'Abono' ? 'text-green-600' : 'text-red-600'}">
									{m.tipo === 'Abono' ? '−' : '+'}{pesos(m.monto)}
								</span>
							</li>
						{/each}
					</ul>
				{/if}
			</div>

			<button onclick={() => (modalCuenta = false)} class="mt-4 w-full rounded-lg border border-slate-300 py-2 text-sm text-slate-600 hover:bg-slate-50">Cerrar</button>
		</div>
	</div>
{/if}
