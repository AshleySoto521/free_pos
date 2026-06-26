<script lang="ts">
	import { api, type ResultadoImport, type FilaCategoriaImport } from '$lib/api';
	import { catalogos, cargarCatalogo, type CatalogoGiro } from '$lib/catalogos';
	import { toast } from '$lib/stores/toast';

	let { onAplicado }: { onAplicado?: (r: ResultadoImport, c: CatalogoGiro) => void } = $props();

	let seleccion = $state<CatalogoGiro | null>(null);
	let preview = $state<FilaCategoriaImport[]>([]);
	let cargando = $state(false);
	let aplicando = $state(false);
	let error = $state('');

	async function seleccionar(c: CatalogoGiro) {
		seleccion = c;
		error = '';
		preview = [];
		cargando = true;
		try {
			preview = await cargarCatalogo(c);
		} catch (e) {
			error = String(e);
		} finally {
			cargando = false;
		}
	}

	async function aplicar() {
		if (!seleccion || preview.length === 0) return;
		aplicando = true;
		error = '';
		try {
			const r = await api.importarCategorias(preview);
			toast(`Catálogo aplicado: ${r.insertados} categorías`);
			onAplicado?.(r, seleccion);
		} catch (e) {
			error = String(e);
			toast('Error al aplicar el catálogo', 'error');
		} finally {
			aplicando = false;
		}
	}
</script>

<div class="space-y-4">
	<div class="grid grid-cols-2 gap-2 sm:grid-cols-3">
		{#each catalogos as c (c.id)}
			<button
				type="button"
				onclick={() => seleccionar(c)}
				class="flex flex-col items-start gap-1 rounded-xl border p-3 text-left transition {seleccion?.id === c.id
					? 'border-indigo-500 bg-indigo-50'
					: 'border-slate-200 hover:border-indigo-300 hover:bg-slate-50'}"
			>
				<span class="text-2xl">{c.emoji}</span>
				<span class="text-sm font-medium text-slate-800">{c.label}</span>
			</button>
		{/each}
	</div>

	{#if error}
		<p class="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>
	{/if}

	{#if seleccion}
		<div class="rounded-xl border border-slate-200 bg-white p-4">
			{#if cargando}
				<p class="text-center text-sm text-slate-400">Cargando catálogo…</p>
			{:else if preview.length > 0}
				<p class="mb-2 text-sm font-medium text-slate-700">
					{seleccion.emoji} {seleccion.label} — <strong>{preview.length}</strong> categorías
				</p>
				<div class="mb-3 flex max-h-44 flex-wrap gap-1.5 overflow-y-auto">
					{#each preview as f (f.categoria)}
						<span class="rounded-full bg-slate-100 px-2.5 py-0.5 text-xs text-slate-600">{f.categoria}</span>
					{/each}
				</div>
				<button
					onclick={aplicar}
					disabled={aplicando}
					class="w-full rounded-lg bg-indigo-600 py-2.5 text-sm font-semibold text-white hover:bg-indigo-700 disabled:opacity-50"
				>
					{aplicando ? 'Aplicando…' : `Usar este catálogo (${preview.length} categorías)`}
				</button>
				<p class="mt-2 text-center text-xs text-slate-400">
					Se agregan a las categorías existentes (las repetidas se omiten).
				</p>
			{/if}
		</div>
	{/if}
</div>
