<script>
	import { onDestroy } from 'svelte';
	import { createEditorViewModel } from '$lib/features/editor/view-model/create-editor-vm.svelte.js';
	import SeriesController from './SeriesController.svelte';
	import RelatedPostsController from './RelatedPostsController.svelte';
	import PostEditorShell from '../editor/PostEditorShell.svelte';

	let { mode = 'create', data, series: initialSeries = [], isOwner = true } = $props();

	const vm = createEditorViewModel({ mode, kind: 'post', data, initialSeries, isOwner });
	onDestroy(() => vm.destroy());
</script>

{#snippet extraFields()}
	<SeriesController
		postId={mode === 'edit' ? vm.entry.id : null}
		bind:series={vm.entry.series}
		bind:seriesSlug={vm.entry.seriesSlug}
		onSelect={(id) => {
			vm.entry.pendingSeriesId = id;
		}}
	/>
	{#if mode === 'edit'}
		<RelatedPostsController postId={vm.entry.id} bind:relatedPosts={vm.entry.relatedPosts} />
	{/if}
{/snippet}

<PostEditorShell
	{vm}
	kind="post"
	coverApiPath={`/api/posts/id/${vm.entry.id}/cover`}
	titleLabel="Title"
	excerptRows={5}
	{extraFields}
/>
