<script lang="ts">
	import { api, type LicenseStatus } from '$lib/api';

	let { estado, onActivada }: { estado: LicenseStatus | null; onActivada: (l: LicenseStatus) => void } =
		$props();

	let clave = $state('');
	let cargando = $state(false);
	let error = $state('');

	async function activar(e: Event) {
		e.preventDefault();
		if (!clave.trim()) return;
		cargando = true;
		error = '';
		try {
			const r = await api.licenciaActivar(clave.trim());
			if (r.valida) onActivada(r);
			else error = r.mensaje || 'No se pudo activar la licencia.';
		} catch (err) {
			error = String(err);
		} finally {
			cargando = false;
		}
	}
</script>

<form onsubmit={activar} class="w-full max-w-sm space-y-5">
	<div class="text-center">
		<img src="/logo_pos.png" alt="AquaPOS" class="mx-auto mb-3 h-16 w-16 rounded-xl object-contain" />
		<h1 class="text-xl font-semibold text-slate-800">Activar AquaPOS</h1>
		<p class="mt-1 text-sm text-slate-500">
			{estado && estado.mensaje && estado.estado !== 'SinActivar'
				? estado.mensaje
				: 'Ingresa tu clave de producto para activar esta computadora.'}
		</p>
	</div>

	<div>
		<label for="clave" class="mb-1 block text-sm font-medium text-slate-700">Clave de producto</label>
		<input
			id="clave"
			bind:value={clave}
			placeholder="POS-XXXX-XXXX-XXXX-XXXX"
			autocomplete="off"
			spellcheck="false"
			class="w-full rounded-lg border-slate-300 font-mono tracking-wider uppercase shadow-sm focus:border-indigo-500 focus:ring-indigo-500"
		/>
	</div>

	{#if error}
		<p class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>
	{/if}

	<button
		type="submit"
		disabled={cargando || !clave.trim()}
		class="w-full rounded-lg bg-indigo-600 px-4 py-2.5 font-medium text-white shadow-sm transition hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-50"
	>
		{cargando ? 'Activando…' : 'Activar'}
	</button>
</form>
