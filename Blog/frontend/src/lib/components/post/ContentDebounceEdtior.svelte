<script>
    import { mediaWithShortcutPlugin } from "$lib/custom-rules";
    import { useDebounce } from "$lib/effects/debounce";
    import MarkdownIt from "markdown-it";
    import { tick } from "svelte";

    const mediaSyntax = /\@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]/g;

    let { delay, onupdate, mediaDictionary, searchMedia, ...rest } = $props();

    let content = $state("");
    let _content = $state("");

    const md = $derived(
        new MarkdownIt().use(mediaWithShortcutPlugin, { mediaDictionary }),
    );

    let debounce = useDebounce(async (_content) => {
        const keys = [..._content.matchAll(mediaSyntax)].map(
            (match) => match[1],
        );
        await searchMedia(keys);
        content = _content;
    }, delay);

    $effect(() => {
        debounce.update(_content);
    });

    $effect(() => {
        $inspect(md);
    });

    $effect(() => {
        $inspect(mediaDictionary);
    });
    $effect(() => {
        onupdate(md.render(content));
    });
</script>

<div {...rest}>
    <textarea
        id="content-editor"
        class="w-full h-full p-2 focus:outline-0 resize-none"
        placeholder="Type here..."
        bind:value={_content}
    ></textarea>
</div>
