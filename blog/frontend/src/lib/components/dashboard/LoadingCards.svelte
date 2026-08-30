<script>
	// Placeholder pulse blocks shown while a page's data loads.
	let { count = 3, grid = false, tall = false } = $props();
</script>

{#if grid}
	<div class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4">
		{#each { length: count } as _, i (i)}
			<div class="bg-white rounded-xl p-4" style:animation-delay={`${i * 100}ms`}>
				<div
					class="aspect-[1.91/1] rounded-lg bg-background/60 animate-pulse"
					style:animation-delay={`${i * 100}ms`}
				></div>
				<div class="h-4 w-3/4 bg-background/60 rounded mt-3 animate-pulse"></div>
				<div class="h-3 w-1/2 bg-background/40 rounded mt-2 animate-pulse"></div>
			</div>
		{/each}
	</div>
{:else}
	<div class="flex flex-col gap-2">
		{#each { length: count } as _, i (i)}
			<div
				class="bg-white rounded-xl p-4 {tall ? 'h-20' : 'h-12'} animate-pulse"
				style:animation-delay={`${i * 100}ms`}
			></div>
		{/each}
	</div>
{/if}
