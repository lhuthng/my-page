<script>
    import {
        mediaWithShortcutPlugin,
        youtubeBlockPlugin,
    } from "$lib/custom-rules";
    import { useDebounce } from "$lib/effects/debounce";
    import MarkdownIt from "markdown-it";
    import mkKatex from "markdown-it-katex";
    import { tick } from "svelte";

    let {
        delay,
        onUpdateRendered,
        onUpdateDraft,
        mediaDictionary,
        searchMedia,
        mediaSyntax,
        registerForceContent,
        forDraft,
        disabled,
        ...rest
    } = $props();

    let content = $state("");
    let _content = $state("");

    registerForceContent((content) => {
        _content = content;
    });

    const md = $derived(
        new MarkdownIt()
            .use(mkKatex)
            .use(mediaWithShortcutPlugin, { mediaDictionary })
            .use(youtubeBlockPlugin),
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
        if (forDraft) onUpdateDraft(_content);
    });

    $effect(() => {
        onUpdateRendered(md.render(content));
    });
</script>

<div {...rest}>
    <textarea
        id="content-editor"
        class="w-full h-full p-2 rounded-sm bg-white/60 focus:outline-0 resize-none custom-scrollbar"
        class:bg-transparent!={disabled}
        placeholder={disabled ? "" : "Type here..."}
        autocorrect="off"
        {disabled}
        bind:value={_content}
    ></textarea>
</div>
