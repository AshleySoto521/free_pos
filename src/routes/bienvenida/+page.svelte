<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api';
	import { session } from '$lib/stores/session';
	import { toast } from '$lib/stores/toast';
	import SelectorCatalogo from '$lib/components/SelectorCatalogo.svelte';

	let paso = $state(1);
	let nombre = $state('');
	let direccion = $state('');
	let telefono = $state('');
	let pie = $state('¡Gracias por su compra!');
	let guardando = $state(false);
	let numCategorias = $state(0);

	onMount(async () => {
		if ($session?.rol !== 'Administrador') {
			goto(resolve('/'));
			return;
		}
		const cfg = await api.listarConfig();
		const v = (k: string) => cfg.find((c) => c.clave === k)?.valor ?? '';
		nombre = v('NombreTienda');
		direccion = v('Direccion');
		telefono = v('Telefono');
		pie = v('PieTicket') || '¡Gracias por su compra!';
		numCategorias = (await api.listarCategorias()).length;
	});

	async function guardarDatos() {
		if (!nombre.trim()) {
			toast('Escribe el nombre del negocio', 'error');
			return;
		}
		guardando = true;
		try {
			await api.guardarConfig([
				{ clave: 'NombreTienda', valor: nombre.trim() },
				{ clave: 'Direccion', valor: direccion.trim() },
				{ clave: 'Telefono', valor: telefono.trim() },
				{ clave: 'PieTicket', valor: pie.trim() },
				{ clave: 'onboarding_completo', valor: '1' } // ya no se vuelve a pedir
			]);
			paso = 2;
		} catch (e) {
			toast(String(e), 'error');
		} finally {
			guardando = false;
		}
	}

	async function trasCatalogo() {
		numCategorias = (await api.listarCategorias()).length;
		paso = 3;
	}
</script>

<div class="flex min-h-screen items-start justify-center overflow-y-auto bg-slate-100 p-6">
	<div class="w-full max-w-lg">
		<!-- Indicador de pasos -->
		<div class="mb-4 flex items-center justify-center gap-2 text-xs font-medium text-slate-500">
			<span class={paso >= 1 ? 'text-indigo-600' : ''}>1. Datos</span>
			<span>›</span>
			<span class={paso >= 2 ? 'text-indigo-600' : ''}>2. Catálogo</span>
			<span>›</span>
			<span class={paso >= 3 ? 'text-indigo-600' : ''}>3. Listo</span>
		</div>

		<div class="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
			{#if paso === 1}
				<div class="mb-4 text-center">
					<img src="/logo_pos.png" alt="Free POS" class="mx-auto mb-2 h-14 w-14 rounded-xl object-contain" />
					<h1 class="text-xl font-semibold text-slate-800">¡Bienvenido! Configura tu negocio</h1>
					<p class="mt-1 text-sm text-slate-500">Estos datos aparecerán en el ticket de venta.</p>
				</div>
				<div class="space-y-3">
					<div>
						<label for="n" class="mb-1 block text-sm font-medium text-slate-700">Nombre del negocio *</label>
						<input id="n" bind:value={nombre} placeholder="Mi Negocio" class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					</div>
					<div>
						<label for="d" class="mb-1 block text-sm font-medium text-slate-700">Dirección</label>
						<input id="d" bind:value={direccion} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					</div>
					<div>
						<label for="t" class="mb-1 block text-sm font-medium text-slate-700">Teléfono</label>
						<input id="t" bind:value={telefono} inputmode="tel" class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					</div>
					<div>
						<label for="p" class="mb-1 block text-sm font-medium text-slate-700">Pie de ticket</label>
						<input id="p" bind:value={pie} class="w-full rounded-lg border-slate-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500" />
					</div>
					<button onclick={guardarDatos} disabled={guardando} class="w-full rounded-lg bg-indigo-600 py-2.5 font-semibold text-white hover:bg-indigo-700 disabled:opacity-50">
						{guardando ? 'Guardando…' : 'Continuar'}
					</button>
				</div>
			{:else if paso === 2}
				<div class="mb-4 text-center">
					<div class="mx-auto mb-2 flex h-14 w-14 items-center justify-center rounded-full bg-indigo-100 text-3xl">🏷️</div>
					<h1 class="text-xl font-semibold text-slate-800">Elige tu catálogo de categorías</h1>
					<p class="mt-1 text-sm text-slate-500">Según el giro de tu negocio. Podrás editarlas o agregar más cuando quieras.</p>
				</div>
				<SelectorCatalogo onAplicado={trasCatalogo} />
				<button onclick={() => (paso = 3)} class="mt-4 w-full rounded-lg border border-slate-300 py-2.5 text-sm font-medium text-slate-600 hover:bg-slate-50">
					Omitir por ahora
				</button>
			{:else}
				<div class="mb-4 text-center">
					<div class="mx-auto mb-2 flex h-14 w-14 items-center justify-center rounded-full bg-green-100 text-3xl">✓</div>
					<h1 class="text-xl font-semibold text-slate-800">¡Todo listo!</h1>
					<p class="mt-1 text-sm text-slate-500">
						Tu negocio tiene <strong>{numCategorias}</strong> categoría{numCategorias === 1 ? '' : 's'}. Ahora puedes:
					</p>
				</div>
				<div class="space-y-2">
					<a href={resolve('/inventario')} class="flex items-center gap-3 rounded-xl border border-slate-200 p-3 hover:border-indigo-300 hover:bg-slate-50">
						<span class="text-2xl">📦</span>
						<span><span class="block font-medium text-slate-800">Dar de alta productos</span><span class="text-xs text-slate-500">Captura tu mercancía o impórtala desde Excel/CSV</span></span>
					</a>
					<a href={resolve('/datos')} class="flex items-center gap-3 rounded-xl border border-slate-200 p-3 hover:border-indigo-300 hover:bg-slate-50">
						<span class="text-2xl">📤</span>
						<span><span class="block font-medium text-slate-800">Importar catálogos</span><span class="text-xs text-slate-500">Productos, clientes y proveedores desde XLSX/CSV</span></span>
					</a>
					<a href={resolve('/proveedores')} class="flex items-center gap-3 rounded-xl border border-slate-200 p-3 hover:border-indigo-300 hover:bg-slate-50">
						<span class="text-2xl">🏷️</span>
						<span><span class="block font-medium text-slate-800">Agregar proveedores</span><span class="text-xs text-slate-500">Tu directorio para reabastecer</span></span>
					</a>
					<a href={resolve('/categorias')} class="flex items-center gap-3 rounded-xl border border-slate-200 p-3 hover:border-indigo-300 hover:bg-slate-50">
						<span class="text-2xl">🏷️</span>
						<span><span class="block font-medium text-slate-800">¿Necesitas más categorías?</span><span class="text-xs text-slate-500">Agrega o edita las que quieras</span></span>
					</a>
				</div>
				<div class="mt-4 flex gap-2">
					<a href={resolve('/ayuda')} class="flex-1 rounded-lg border border-slate-300 py-2.5 text-center text-sm font-medium text-slate-600 hover:bg-slate-50">📖 Ver guía</a>
					<button onclick={() => goto(resolve('/'))} class="flex-1 rounded-lg bg-indigo-600 py-2.5 font-semibold text-white hover:bg-indigo-700">Ir al inicio</button>
				</div>
			{/if}
		</div>
	</div>
</div>
