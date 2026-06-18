<script>
	import { el } from '$lib/dom/elements.svelte.js';
	import Portal from './Portal.svelte';

	let { visible = true, children } = $props();
</script>

<Portal
	class={`full bg-dark/50 block overflow-hidden ${!children ? 'opacity-0 pointer-events-none' : 'opacity-100 pointer-events-auto'}`}
	{visible}
	target={el.pbody}
>
	{@render children?.()}
</Portal>
