<script>
	import { fade, fly } from 'svelte/transition';

	let { commandState, pickCommandItem } = $props();
</script>

{#if commandState.items.length > 0}
	<div in:fade={{ duration: 100 }}>
		{#each commandState.items as gif, index (gif.id)}
			<button
				in:fly={{ y: 20, duration: 200 + index * 100 }}
				type="button"
				class:bg-primary-20={index === commandState.selected}
				onmousedown={(event) => {
					event.preventDefault();
					pickCommandItem(gif);
				}}
			>
				<img src={gif.images.fixed_height.url} alt={gif.title} loading="lazy" />
				<p>{gif.title || 'GIF'}</p>
			</button>
		{/each}
	</div>
{/if}

<style lang="postcss">
	@reference "../../../../app.css";
	div {
		@apply grid gap-2;
		grid-template-columns: repeat(auto-fit, minmax(6rem, 1fr));

		& > button {
			@apply flex flex-col items-center gap-2 rounded-lg border border-primary/15 p-1 text-left hover:bg-primary-20;
			& > img {
				@apply h-24 w-24 rounded object-cover;
			}
			& > p {
				@apply line-clamp-4 text-center text-sm leading-tight font-medium;
			}
		}
	}
</style>
