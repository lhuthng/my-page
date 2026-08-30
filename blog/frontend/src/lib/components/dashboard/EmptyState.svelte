<script>
	import Smug from '$lib/components/svgs/emotes/Smug.svelte';

	let { message, hint = '', mascot = false, actions, children } = $props();
</script>

<div
	class="flex flex-col items-center gap-3 py-12 text-dark/40 border-2 border-dashed border-dark/10 rounded-xl text-center px-4"
>
	{#if mascot}
		<Smug class="w-14 h-10 animate-wave" />
	{/if}
	<p class="text-lg">{message}</p>
	{#if hint}
		<p class="text-sm text-dark/30 max-w-sm">{hint}</p>
	{/if}
	{#if actions}
		{@render actions()}
	{/if}
	{@render children?.()}
</div>
