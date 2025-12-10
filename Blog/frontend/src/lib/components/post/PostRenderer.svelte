<script>
    import { mediaWithShortcutPlugin } from "$lib/custom-rules";
    import { useDebounce } from "$lib/effects/debounce";
    import MarkdownIt from "markdown-it";
    import Post from "./Post.svelte";

    let { text, mediaDictionary } = $props();

    let md = $derived(
        new MarkdownIt().use(mediaWithShortcutPlugin, { mediaDictionary }),
    );

    let renderedText = $derived(md.render(text));
</script>

<Post content={renderedText} />
