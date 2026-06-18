<script>
	import Smug from '../svgs/emotes/Smug.svelte';

	let {
		isLoadingMore = false,
		hasMore = true,
		onclick,
		label = 'item',
		intro = false,
		delay = 0
	} = $props();
</script>

<li
	class="flex gap-2 md:flex-col justify-center items-center full min-w-22 sm:min-w-26 min-h-22 sm:min-h-26 md:min-w-34 md:min-h-34 rounded-lg border-2 border-dashed"
	class:animate-fly-in={intro}
	style:--delay={`${delay}ms`}
>
	<Smug
		class="relative w-24 hover:scale-110 origin-[60%_96%] duration-100 transition-[scale,rotate] animate-wave"
	/>
	<div class="duo-btn" data-duo-color={!isLoadingMore && hasMore ? 'blue' : 'primary'}>
		<button type="button" class="no-underline!" disabled={isLoadingMore || !hasMore} {onclick}>
			{#if isLoadingMore}
				Loading more {label}s...
			{:else if hasMore}
				Load more {label}s
			{:else}
				No more
			{/if}
		</button>
	</div>
</li>
