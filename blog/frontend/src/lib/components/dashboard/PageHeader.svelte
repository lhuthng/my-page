<script>
	let { title, count, actions } = $props();
</script>

<header class="flex flex-wrap items-center justify-between gap-2">
	<h1 class="text-2xl font-semibold">
		{title}{#if count !== undefined}<span class="text-dark/40 text-lg font-normal">
				({count})
			</span>{/if}
	</h1>
	{#if actions}
		{@render actions()}
	{/if}
</header>
