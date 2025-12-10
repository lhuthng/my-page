<script>
	import { mediaWithShortcutPlugin } from "$lib/custom-rules";
	import MarkdownIt from "markdown-it/dist/index.cjs.js";
	import { onMount } from "svelte";

	const { mediaDictionary } = $props();

	let md = $derived.by(() =>
		new MarkdownIt().use(mediaWithShortcutPlugin, { mediaDictionary }),
	);

	let markdown = $state("");
	let renderedMarkdown = $derived(md.render(markdown));
</script>

<div class="flex">
	<textarea class="bg-red-100" bind:value={markdown}></textarea>
	<div class="rendered-markdown grow bg-blue-100">
		{@html renderedMarkdown}
	</div>
</div>
