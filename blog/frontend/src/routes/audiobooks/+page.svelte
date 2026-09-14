<script>
	import AudiobookCard from '$lib/components/audio/AudiobookCard.svelte';
	import BackButton from '$lib/components/ui/BackButton.svelte';
	import EmptyState from '$lib/components/dashboard/EmptyState.svelte';
	import { SITE_OG_IMAGE } from '$lib/config/site.js';

	let { data } = $props();

	const audiobooks = $derived(data.audiobooks ?? []);
	const heading = $derived(
		data.tag
			? `Audiobooks tagged “${data.tag}”`
			: data.term
				? `Audiobooks matching “${data.term}”`
				: 'Audiobooks'
	);
</script>

<svelte:head>
	<title>Audiobooks | Huu Thang's Blog</title>
	<meta
		name="description"
		content="Listen to audiobooks and translated works, chapter by chapter, with continuous playback."
	/>
	<meta property="og:title" content="Audiobooks | Huu Thang's Blog" />
	<meta
		property="og:description"
		content="Listen to audiobooks and translated works, chapter by chapter, with continuous playback."
	/>
	<meta property="og:type" content="website" />
	<meta property="og:image" content={SITE_OG_IMAGE} />
	<meta name="twitter:card" content="summary_large_image" />
</svelte:head>

<section class="flex flex-col gap-4 *:bg-white *:rounded-xl *:p-4 pb-8">
	<div class="flex flex-col gap-2">
		<BackButton href="/" text="Home" />
		<div class="flex items-end justify-between gap-2 flex-wrap">
			<h1 class="text-2xl font-semibold">{heading}</h1>
			<span class="text-sm text-dark/50">
				{audiobooks.length} title{audiobooks.length === 1 ? '' : 's'}
			</span>
		</div>
	</div>

	{#if audiobooks.length === 0}
		<div>
			<EmptyState
				message="No audiobooks published yet."
				hint={data.tag || data.term
					? 'Try a different search or tag.'
					: 'Published audiobooks will appear here.'}
			/>
		</div>
	{:else}
		<ul class="grid grid-cols-1 xl:grid-cols-2 gap-3 !bg-transparent !p-0">
			{#each audiobooks as audiobook (audiobook.id)}
				<AudiobookCard {audiobook} />
			{/each}
		</ul>
	{/if}
</section>
