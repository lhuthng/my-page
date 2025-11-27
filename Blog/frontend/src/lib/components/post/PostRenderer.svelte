<script>
    import { mediaWithShortcutPlugin } from "$lib/custom-rules";
    import { useDebounce } from "$lib/effects/debounce";
    import MarkdownIt from "markdown-it";

    let { text, mediaDictionary } = $props();

    let md = $derived(
        new MarkdownIt().use(mediaWithShortcutPlugin, { mediaDictionary }),
    );

    let renderedText = $derived(md.render(text));
</script>

<div class="rendered-markdown">
    {@html renderedText}
</div>

<style lang="postcss">
    @reference "tailwindcss";

    :global(.rendered-markdown) {
        & h1 {
        }
        & img {
            @apply rounded-lg mx-auto;
        }
    }
</style>
