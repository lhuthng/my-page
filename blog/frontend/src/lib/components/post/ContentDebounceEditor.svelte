<script>
	import {
		appBlockPlugin,
		codeHighlightPlugin,
		iframeBlockPlugin,
		mediaWithShortcutPlugin,
		namedContainerPlugin,
		revealPlugin,
		slugify,
		youtubeBlockPlugin,
		kaomojiPlugin
	} from '$lib/custom-rules';
	import { useDebounce } from '$lib/utils/debounce';
	import MarkdownIt from 'markdown-it';
	import mkKatex from 'markdown-it-katex';
	import anchor from 'markdown-it-anchor';
	import { tick } from 'svelte';

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

	let content = $state('');
	let _content = $state('');

	registerForceContent((content) => {
		_content = content;
	});

	const md = $derived(
		new MarkdownIt()
			.use(mkKatex)
			.use(mediaWithShortcutPlugin, { mediaDictionary })
			.use(iframeBlockPlugin)
			.use(youtubeBlockPlugin)
			.use(appBlockPlugin, { mediaDictionary })
			.use(revealPlugin)
			.use(namedContainerPlugin)
			.use(codeHighlightPlugin)
			.use(kaomojiPlugin)
			.use(anchor, { slugify })
	);

	const lottieAppSyntax = /:::app\s+lottie\s+([^\s]+)/g;
	const collectMediaKeys = (text) => [
		...[...text.matchAll(mediaSyntax)].map((match) => match[1]),
		...[...text.matchAll(lottieAppSyntax)].map((match) => match[1])
	];

	let debounce = useDebounce(async (_content) => {
		const keys = collectMediaKeys(_content);
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
		placeholder={disabled ? '' : 'Type here...'}
		autocorrect="off"
		{disabled}
		bind:value={_content}></textarea>
</div>
