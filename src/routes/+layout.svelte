<script lang="ts">
	import './layout.css';
	import Gate from '$lib/components/Gate.svelte';
	import Toast from '$lib/components/Toast.svelte';
	import { APP_VERSION } from '$lib/version';

	let { children } = $props();

	const year = new Date().getFullYear();
</script>

<div class="flex min-h-screen flex-col">
	<div class="flex-1">
		<Gate>
			{@render children()}
		</Gate>
	</div>

	<footer class="border-t border-slate-200 bg-white px-6 py-4 text-center text-xs text-slate-500">
		<p>
			Soporte técnico, compras o dudas:
			<a href="mailto:contactoaquastudio@gmail.com" class="font-medium text-slate-600 hover:underline">
				contactoaquastudio@gmail.com
			</a>
		</p>
		<p class="mt-0.5">
			© Aqua Studio {year} — Todos los derechos reservados
			<span class="text-slate-400">· AquaPOS v{APP_VERSION}</span>
		</p>
	</footer>
</div>

<Toast />
