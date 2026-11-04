<script>
	import HTMLApp from './HTMLApp.svelte';

	let { name, type, width, height, config, temp, missing = false } = $props();

	const missingLabel = $derived(
		type === 'game' ? 'This game is no longer available.' : 'This project is no longer available.'
	);

	let GLBDemo = $state();
	let LottieStateSwitcher = $state();
	let JsDosApp = $state();
	let V86App = $state();
	let GameApp = $state();
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
		// A whole game by slug, whichever launcher it uses — `:::app game my-game`.
		if (type === 'game') {
			import('./GameApp.svelte').then((m) => (GameApp = m.default));
		}
	});
</script>

{#if missing}
	<div
		class="mx-auto grid place-items-center rounded-xl bg-background p-4 text-center text-dark/70"
		style:width
		style:height
	>
		<p>{missingLabel}</p>
	</div>
{:else if type === 'html' || type === 'project'}
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
{:else if type === 'game'}
	{#if GameApp}
		<GameApp {name} {width} {height} />
	{/if}
{/if}
