<script>
	import ImageManager from "$lib/components/ImageManager.svelte";
	import MarkdownRenderer from "$lib/components/MarkdownRenderer.svelte";

	let mediaDictionary = $state({});
</script>

<ImageManager bind:mediaDictionary />
<MarkdownRenderer {mediaDictionary} />

<style lang="postcss">
	@reference "tailwindcss";

	:global(.rendered-markdown) {
		@apply bg-red-200;

		& h1 {
			@apply text-red-500;
		}
	}
</style>
