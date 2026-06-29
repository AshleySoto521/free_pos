<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { APP_VERSION } from '$lib/version';

	function imprimir() {
		window.print();
	}
</script>

<div class="min-h-screen bg-slate-100">
	<header class="flex items-center justify-between border-b border-slate-200 bg-white px-5 py-3 print:hidden">
		<div class="flex items-center gap-3">
			<button onclick={() => goto(resolve('/'))} class="rounded-lg px-2 py-1 text-slate-500 hover:bg-slate-100" aria-label="Volver">←</button>
			<h1 class="text-lg font-semibold text-slate-800">Guía de uso</h1>
		</div>
		<button onclick={imprimir} class="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-semibold text-white hover:bg-indigo-700">
			🖨️ Imprimir / Guardar PDF
		</button>
	</header>

	<main class="mx-auto max-w-3xl p-6">
		<article id="guia-imprimible" class="prose prose-slate max-w-none rounded-2xl border border-slate-200 bg-white p-8 shadow-sm">
			<h1>AquaPOS — Guía de uso</h1>
			<p>Punto de venta para tu negocio. Esta guía resume cómo operarlo día a día.</p>
			<p><strong>Versión {APP_VERSION}</strong></p>

			<h2>Roles</h2>
			<ul>
				<li><strong>Administrador:</strong> acceso total — productos, inventario, compras, proveedores, clientes, usuarios, catálogos, reportes y bitácora.</li>
				<li><strong>Cajero:</strong> puede iniciar sesión, <strong>abrir caja, vender</strong> y registrar abonos/clientes; no modifica inventario ni catálogos.</li>
			</ul>

			<h2>Primeros pasos</h2>
			<ol>
				<li>Crea el usuario administrador (configuración inicial).</li>
				<li>Captura los <strong>datos del negocio</strong> (aparecen en el ticket).</li>
				<li>Elige tu <strong>moneda</strong> y tu <strong>modo de venta</strong> (productos, servicios o ambos) en el asistente o en Configuración.</li>
				<li>Elige un <strong>catálogo de categorías</strong> según tu giro (asistente inicial o Catálogos → Categorías → 📚 Catálogo por giro).</li>
				<li>Carga tu inventario inicial en Catálogos → Importar/Exportar → <strong>🏁 Iniciar inventario</strong> (solo al inaugurar). Después el stock entra por <strong>Compras</strong>.</li>
			</ol>

			<h2>Vender 🛒</h2>
			<ul>
				<li><strong>Primero abre la caja</strong>: cuenta billetes y monedas; ese total es tu fondo inicial.</li>
				<li>Escanea el código de barras (o búscalo) para agregar productos al ticket.</li>
				<li>Elige el cliente (o “Público en General”) y el método de pago. En efectivo se calcula el cambio.</li>
				<li>Para <strong>fiado</strong>, selecciona un cliente registrado.</li>
				<li>Al cobrar, imprime el ticket o guárdalo en PDF.</li>
			</ul>

			<h2>Corte de caja 💵</h2>
			<ul>
				<li>Al <strong>abrir</strong> y <strong>cerrar</strong> el turno se cuenta el efectivo por denominación.</li>
				<li>El cierre compara lo <em>esperado</em> (fondo + ventas en efectivo + ingresos − retiros) contra lo <em>contado</em> y muestra la diferencia.</li>
				<li>Usa <strong>Ingreso/Retiro</strong> para movimientos de efectivo que no son ventas.</li>
			</ul>

			<h2>Inventario 📦</h2>
			<ul>
				<li>Crea y edita productos (precio, código, categoría, unidad, stock mínimo, si se vende por peso o maneja caducidad).</li>
				<li>El producto se crea con <strong>0 existencia</strong>: el stock entra por <strong>Compras</strong> (o por la carga inicial), nunca en el alta.</li>
				<li><strong>Ajustar</strong> existencias: <em>Conteo físico</em> (existencia real) o <em>Merma/Baja</em>; el <strong>motivo es obligatorio</strong>.</li>
				<li>Costeo <strong>PEPS</strong> automático; revisa el <strong>kardex valorizado</strong> de cada producto (botón Historial).</li>
				<li>Las existencias bajas se marcan en rojo. La caducidad/lote avisa de próximos a vencer.</li>
			</ul>

			<h2>Clientes y fiado 🧾</h2>
			<ul>
				<li>Registra clientes con <strong>nombre y teléfono obligatorios</strong> y correo opcional.</li>
				<li>Consulta su <strong>estado de cuenta</strong> (cargos y abonos) y registra <strong>abonos</strong> para bajar su deuda.</li>
			</ul>

			<h2>Compras y proveedores 🚚</h2>
			<ul>
				<li>Registra compras para <strong>reabastecer</strong>: suma al inventario y crea la capa de costo PEPS (el costo de referencia se actualiza solo).</li>
				<li>Agrega varios productos a la vez o <strong>importa la factura</strong> (Excel/CSV); si trae precio de venta, también lo actualiza.</li>
				<li><strong>No se duplican facturas</strong> (aunque no tengan folio) y solo se registran facturas completas.</li>
				<li>Directorio de proveedores con teléfono obligatorio + correo opcional.</li>
			</ul>

			<h2>Reportes 📈</h2>
			<ul>
				<li>Genera reportes de <strong>ventas, inventario, cortes, utilidad e inventario valorizado</strong> en Excel (XLSX), PDF o ambos.</li>
			</ul>

			<h2>Catálogos ⚙️</h2>
			<ul>
				<li>Categorías, unidades, métodos de pago, <strong>monedas y denominaciones</strong> y datos del negocio.</li>
			</ul>

			<h2>Importar / Exportar 📥</h2>
			<ul>
				<li>Cada sección tiene un botón <strong>⬇️ Descargar plantilla .csv</strong>: bájala, llénala en Excel y súbela.</li>
				<li>Tres formas de cargar productos: <strong>Agregar productos</strong> (nuevos, sin stock), <strong>🏁 Iniciar inventario</strong> (carga inicial) y <strong>💲 Actualizar precios</strong> (en masa).</li>
				<li>Si falta un dato, no se importa nada y te dice qué corregir.</li>
			</ul>

			<h2>Bitácora 📋</h2>
			<ul>
				<li>Registro de quién hizo qué (ventas, cancelaciones, altas, cierres…). Solo administradores.</li>
			</ul>

			<h2>Licencia</h2>
			<ul>
				<li>El sistema incluye una <strong>prueba</strong> al inicio; después se activa con una clave de producto.</li>
			</ul>

			<hr />
			<p><em>AquaPOS v{APP_VERSION} — Aqua Studio.</em></p>
		</article>
	</main>
</div>

<style>
	@media print {
		:global(body *) {
			visibility: hidden !important;
		}
		:global(#guia-imprimible),
		:global(#guia-imprimible *) {
			visibility: visible !important;
		}
		:global(#guia-imprimible) {
			position: absolute;
			left: 0;
			top: 0;
			width: 100%;
			border: none !important;
			box-shadow: none !important;
		}
	}
</style>
