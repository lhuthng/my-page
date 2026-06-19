<script>
	import HTMLApp from './HTMLApp.svelte';
	import LottieStateSwitcher from './LottieStateSwitcher.svelte';

	let { name, type, width, height, config, temp } = $props();

	let GLBDemo = $state();
	$effect(() => {
		if (type === 'glb-demo') {
			import('./GLBDemo.svelte').then((m) => (GLBDemo = m.default));
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
	<LottieStateSwitcher {name} {states} {width} {height} src={temp} />
{/if}
