<script>
	import { api } from '$lib/api/client';
	import { onMount, untrack } from 'svelte';
	import { flip } from 'svelte/animate';
	import FetchMore from '$lib/components/home/FetchMore.svelte';
	import BigPostCard from '$lib/components/home/BigPostCard.svelte';
	import GridExpander from '$lib/components/shell/GridExpander.svelte';
	import BackButton from '$lib/components/ui/BackButton.svelte';
	import { SITE_OG_IMAGE } from '$lib/config/site.js';

	let { data } = $props();

	const limit = $derived(untrack(() => data.firstOffset ?? 5));
	const itemDelay = 45;
	const imageUrl = SITE_OG_IMAGE;

	let batchId = 0;

	let games = $state(
		untrack(() =>
			data.status === 'success'
				? (data.games ?? []).map((game, index) => ({
						...game,
						_batchId: batchId,
						_introDelay: index * itemDelay
					}))
				: []
		)
	);

	let length = $derived(games.length);
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
			const payload = await api.get(`games/latest?limit=${limit}&offset=${games.length}`, {
				auth: false
			});

			batchId += 1;

			const newGames = (payload.games ?? []).map((game, index) => ({
				...game,
				_batchId: batchId,
				_introDelay: index * itemDelay
			}));

			games = [...games, ...newGames];
			hasMore = Boolean(payload.has_more);
		} catch {
			loadError = 'Could not load more games right now.';
		} finally {
			isLoadingMore = false;
		}
	};
</script>

<svelte:head>
	<title>Games | Huu Thang's Blog</title>
	<meta property="og:title" content="Games | Huu Thang's Blog" />
	<meta name="description" content="Playable games and their stories." />
	<meta property="og:description" content="Playable games and their stories." />
	<meta property="og:type" content="website" />
	<meta property="og:image" content={imageUrl} />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content="Games | Huu Thang's Blog" />
	<meta name="twitter:description" content="Playable games and their stories." />
	<meta name="twitter:image" content={imageUrl} />
</svelte:head>

<div class="bg-white rounded-xl mb-2 lg:mb-4">
	<div class="px-4 pt-4 space-y-2">
		<BackButton href="/" text="Home" />
		<h1 class="text-2xl font-semibold">Games</h1>
	</div>
	<GridExpander
		class="p-4"
		expanded={(hydrated && expanded) || !hydrated}
		duration={hydrated ? '1s' : '0ms'}
	>
		{#if data.status !== 'success'}
			<div class="text-dark/60">Could not load games right now.</div>
		{:else if length === 0}
			<div class="text-dark/60">No published games yet.</div>
		{:else}
			<ul class="grid grid-cols-[repeat(auto-fill,minmax(15rem,1fr))] gap-4">
				{#each games as game (game.id)}
					<li
						animate:flip={{ duration: 250 }}
						class:animate-fly-in={hydrated}
						style:--delay={`${game._introDelay}ms`}
					>
						<BigPostCard
							id={game.id}
							title={game.title}
							slug={game.slug}
							excerpt={game.excerpt}
							status={game.status}
							author={{ name: game.author_name, slug: game.author_slug }}
							tags={game.tag_slugs}
							src={game.url}
							stats={game.stats}
							readingTime={game.reading_time_minutes}
							routePrefix="/games"
							dashboardPrefix="/dashboard/games/id"
							coverMediaType={game.cover_media_type}
						/>
					</li>
				{/each}

				{#if expanded}
					<FetchMore
						{isLoadingMore}
						{hasMore}
						label="game"
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
