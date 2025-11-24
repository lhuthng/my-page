<script>
    import { mediaWithShortcutPlugin } from "$lib/custom-rules";
    import { useDebounce } from "$lib/effects/debounce";
    import MarkdownIt from "markdown-it";

    let { text, mediaDictionary, mediaSyntax, handlers } = $props();

    let debounce = useDebounce(async (text) => {
        // Match all [tag:value]
        const keys = [...text.matchAll(mediaSyntax)].map((match) => match[1]);
        handlers.findMedia(keys);
    }, 300);

    let md = $derived(
        new MarkdownIt().use(mediaWithShortcutPlugin, { mediaDictionary }),
    );

    let renderedText = $derived(md.render(text));

    $effect(() => {
        debounce.update(text);
    });
</script>

<div class="rendered-markdown">
    {@html renderedText}
</div>

<style lang="postcss">
    @reference "tailwindcss";

    :global(.rendered-markdown) {
        & h1 {
            @apply text-red-500;
        }
    }
</style>
