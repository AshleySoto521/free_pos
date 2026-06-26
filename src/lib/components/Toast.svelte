<script lang="ts">
	import { fly } from 'svelte/transition';
	import { toasts } from '$lib/stores/toast';
</script>

<div class="pointer-events-none fixed right-4 bottom-4 z-50 flex flex-col gap-2">
	{#each $toasts as t (t.id)}
		<div
			transition:fly={{ y: 12, duration: 150 }}
			class="rounded-lg px-4 py-2.5 text-sm font-medium text-white shadow-lg {t.tipo === 'error'
				? 'bg-red-600'
				: 'bg-slate-800'}"
		>
			{t.tipo === 'error' ? '⚠️ ' : '✓ '}{t.msg}
		</div>
	{/each}
</div>
