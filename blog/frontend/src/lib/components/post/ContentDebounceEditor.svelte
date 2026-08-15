<script>
	import { onDestroy } from 'svelte';
	import { useDebounce } from '$lib/utils/debounce';
	import { createMarkdownRenderer, renderBody } from '$lib/features/editor/markdown/renderer.js';
	import { collectMediaKeys } from '$lib/features/editor/media/references.js';

	let {
		value = $bindable(''),
		delay = 500,
		mediaDictionary,
		disabled = false,
		onRenderedUpdate = () => {},
		onKeysChanged = () => {},
		...rest
	} = $props();

	// Built once — this used to be a `$derived`, which rebuilt the entire
	// parser and all ten plugins on every dependency change.
	const md = createMarkdownRenderer();

	let debouncedValue = $state(value);

	const debounce = useDebounce((next) => {
		debouncedValue = next;
		onKeysChanged(collectMediaKeys(next));
	}, delay);

	// One-directional: reads `value` (which only the textarea's own `bind:value`
	// or the parent ever write) and schedules a local update. Nothing here
	// writes back to `value`, so there is no cycle through the parent.
	$effect(() => {
		debounce.update(value);
	});

	$effect(() => {
		onRenderedUpdate(renderBody(md, debouncedValue, mediaDictionary));
	});

	onDestroy(() => debounce.destroy());
</script>

<div {...rest}>
	<textarea
		id="content-editor"
		class="w-full h-full rounded-sm p-1 focus:outline-0 resize-none custom-scrollbar"
		style="background-color: {disabled
			? 'color-mix(in oklab,var(--color-primary) 30%,transparent)'
			: 'transparent'};"
		placeholder={disabled ? '' : 'Type here...'}
		autocorrect="off"
		{disabled}
		bind:value></textarea>
</div>
