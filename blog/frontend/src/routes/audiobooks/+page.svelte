<script>
	import { api } from '$lib/api/client';
	import { onMount, untrack } from 'svelte';
	import { flip } from 'svelte/animate';
	import FetchMore from '$lib/components/home/FetchMore.svelte';
	import AudiobookCard from '$lib/components/audio/AudiobookCard.svelte';
	import GridExpander from '$lib/components/shell/GridExpander.svelte';
	import BackButton from '$lib/components/ui/BackButton.svelte';
	import { SITE_OG_IMAGE } from '$lib/config/site.js';

	let { data } = $props();

	const limit = $derived(untrack(() => data.firstOffset ?? 12));
	const itemDelay = 45;
	const imageUrl = SITE_OG_IMAGE;

	let batchId = 0;

	let audiobooks = $state(
		untrack(() =>
			data.status === 'success'
				? (data.audiobooks ?? []).map((audiobook, index) => ({
						...audiobook,
						_batchId: batchId,
						_introDelay: index * itemDelay
					}))
				: []
		)
	);

	const heading = $derived(
		data.tag
			? `Audiobooks tagged “${data.tag}”`
			: data.term
				? `Audiobooks matching “${data.term}”`
				: 'Audiobooks'
	);

	let length = $derived(audiobooks.length);
	let hasMore = $state(() => Boolean(data.has_more));
	let isLoadingMore = $state(false);
	let loadError = $state('');

	let hydrated = $state(false);
	let expanded = $state(false);

	onMount(() => {
		hydrated = true;

		requestAnimationFrame(() => {
			expanded = true;
		});
	});

	const fetchMore = async () => {
		if (isLoadingMore || !hasMore) return;

		isLoadingMore = true;
		loadError = '';

		try {
			const params = new URLSearchParams();
			if (data.tag) params.set('tag', data.tag);
			if (data.term) params.set('term', data.term);
			params.set('limit', String(limit + 1));
			params.set('offset', String(audiobooks.length));

			const payload = await api.get(`audiobooks/public/all?${params}`, {
				auth: false
			});

			batchId += 1;

			const rows = payload.audiobooks ?? [];
			const more = rows.length > limit;

			const newAudiobooks = rows.slice(0, limit).map((audiobook, index) => ({
				...audiobook,
				_batchId: batchId,
				_introDelay: index * itemDelay
			}));

			audiobooks = [...audiobooks, ...newAudiobooks];
			hasMore = more;
		} catch {
			loadError = 'Could not load more audiobooks right now.';
		} finally {
			isLoadingMore = false;
		}
	};
</script>

<svelte:head>
	<title>{heading} | Huu Thang's Blog</title>
	<meta
		name="description"
		content="Listen to audiobooks and translated works, chapter by chapter, with continuous playback."
	/>
	<meta property="og:title" content="{heading} | Huu Thang's Blog" />
	<meta
		property="og:description"
		content="Listen to audiobooks and translated works, chapter by chapter, with continuous playback."
	/>
	<meta property="og:type" content="website" />
	<meta property="og:image" content={imageUrl} />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content="{heading} | Huu Thang's Blog" />
	<meta
		name="twitter:description"
		content="Listen to audiobooks and translated works, chapter by chapter, with continuous playback."
	/>
	<meta name="twitter:image" content={imageUrl} />
</svelte:head>

<div class="bg-white rounded-xl mb-2 lg:mb-4">
	<div class="px-4 pt-4 space-y-2">
		<BackButton href="/" text="Home" />
		<div class="flex items-end justify-between gap-2 flex-wrap">
			<h1 class="text-2xl font-semibold">{heading}</h1>
			<span class="text-sm text-dark/50">
				{audiobooks.length} title{audiobooks.length === 1 ? '' : 's'}
			</span>
		</div>
	</div>
	<GridExpander
		class="p-4"
		expanded={(hydrated && expanded) || !hydrated}
		duration={hydrated ? '1s' : '0ms'}
	>
		{#if data.status !== 'success'}
			<div class="text-dark/60">Could not load audiobooks right now.</div>
		{:else if length === 0}
			<div class="text-dark/60">
				{data.tag || data.term
					? 'No audiobooks found. Try a different search or tag.'
					: 'No published audiobooks yet.'}
			</div>
		{:else}
			<ul class="grid grid-cols-[repeat(auto-fill,minmax(15rem,1fr))] gap-4">
				{#each audiobooks as audiobook (audiobook.id)}
					<li
						animate:flip={{ duration: 250 }}
						class:animate-fly-in={hydrated}
						style:--delay={`${audiobook._introDelay}ms`}
					>
						<AudiobookCard {audiobook} />
					</li>
				{/each}

				{#if expanded}
					<FetchMore
						{isLoadingMore}
						{hasMore}
						label="audiobook"
						intro={hydrated}
						delay={length * itemDelay}
						onclick={fetchMore}
					/>
				{/if}
			</ul>
		{/if}

		{#if loadError}
			<p class="text-sm text-dark/60">{loadError}</p>
		{/if}
	</GridExpander>
</div>
