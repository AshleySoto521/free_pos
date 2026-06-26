<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import Activar from '$lib/components/Activar.svelte';
	import { licencia } from '$lib/stores/session';
	import type { LicenseStatus } from '$lib/api';

	function onActivada(lic: LicenseStatus) {
		licencia.set(lic);
		goto(resolve('/'));
	}
</script>

<div class="flex min-h-screen items-center justify-center bg-slate-100 p-6">
	<div class="w-full max-w-sm">
		<button onclick={() => goto(resolve('/'))} class="mb-4 text-sm text-slate-500 hover:text-slate-700">
			← Volver
		</button>
		<div class="rounded-2xl bg-white p-8 shadow-sm">
			<Activar estado={$licencia} {onActivada} />
		</div>
	</div>
</div>
