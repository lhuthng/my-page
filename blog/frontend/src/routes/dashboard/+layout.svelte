<script>
	let { children } = $props();
</script>

<svelte:head>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<!-- Navigation lives in the lg+ rail and, below lg, in the header's burger
     menu — this layout only frames the page content. -->
<div class="flex flex-col gap-2 lg:gap-4 pb-8">
	{@render children?.()}
</div>
