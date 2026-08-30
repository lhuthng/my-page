<script>
	import HTMLApp from './HTMLApp.svelte';

	let { name, type, width, height, config, temp } = $props();

	let GLBDemo = $state();
	let LottieStateSwitcher = $state();
	let JsDosApp = $state();
	let V86App = $state();
	$effect(() => {
		if (type === 'glb-demo') {
			import('./GLBDemo.svelte').then((m) => (GLBDemo = m.default));
		}
		if (type === 'lottie') {
			import('./LottieStateSwitcher.svelte').then((m) => (LottieStateSwitcher = m.default));
		}
		if (type === 'jsdos') {
			import('./JsDosApp.svelte').then((m) => (JsDosApp = m.default));
		}
		if (type === 'v86') {
			import('./V86App.svelte').then((m) => (V86App = m.default));
		}
	});
</script>

{#if type === 'html' || type === 'project'}
	<HTMLApp {name} {type} {width} {height} {config} />
{:else if type === 'glb-demo'}
	{#if GLBDemo}
		<GLBDemo {name} {type} {width} {height} {config} {temp} />
	{/if}
{:else if type === 'lottie'}
	{@const states = (config ?? '')
		.split('-')
		.reduce((a, _, i, arr) => (i % 2 ? [...a, [arr[i - 1], +arr[i]]] : a), [])}
	{#if LottieStateSwitcher}
		<LottieStateSwitcher {name} {states} {width} {height} src={temp} />
	{/if}
{:else if type === 'jsdos'}
	{#if JsDosApp}
		<JsDosApp {name} {width} {height} />
	{/if}
{:else if type === 'v86'}
	{#if V86App}
		<V86App {name} {width} {height} />
	{/if}
{/if}
