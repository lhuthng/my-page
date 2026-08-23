<script>
	import { api } from '$lib/api/client';
	import PostCard from '$lib/components/home/PostCard.svelte';
	import { onMount, untrack } from 'svelte';
	import { fly, fade } from 'svelte/transition';

	let { data } = $props();

	let games = $state(untrack(() => data).games ?? []);
	let loading = $state(false);
	let loadingMore = $state(false);
	let error = $state(null);
	const LIMIT = 9;
	let hasMore = $state(untrack(() => data).hasMore ?? false);

	async function fetchMore() {
		if (loadingMore || !hasMore) return;
		loadingMore = true;
		error = null;

		try {
			const result = await api.get(`games/all?limit=${LIMIT}&offset=${games.length}`);
			const items = result.games ?? [];
			games = [...games, ...items];
			hasMore = Boolean(result.has_more);
		} catch (e) {
			error = e.message;
		} finally {
			loadingMore = false;
		}
	}

	onMount(() => {
		if (games.length === 0) {
			loading = true;
			api
				.get(`games/all?limit=${LIMIT}&offset=0`)
				.then((result) => {
					games = result.games ?? [];
					hasMore = Boolean(result.has_more);
				})
				.catch((e) => (error = e.message))
				.finally(() => (loading = false));
		}
	});
</script>

<svelte:head>
	<title>Games - Dashboard | Huu Thang's Blog</title>
</svelte:head>

<section class="flex flex-col gap-4 *:bg-white *:rounded-xl *:p-4 pb-8">
	<div class="flex flex-col gap-4">
		<!-- Header -->
		<div class="flex flex-wrap items-center justify-between gap-3">
			<h1 class="text-2xl font-semibold">
				Games
				<span class="text-dark/40 text-lg font-normal">({games.length})</span>
			</h1>
			<div class="w-fit duo-btn" data-duo-color="green">
				<a href="/dashboard/games/new">New Game</a>
			</div>
		</div>

		<!-- Content -->
		{#if loading}
			<div class="flex justify-center items-center py-12 text-dark/40">Loading…</div>
		{:else if error}
			<p class="text-accent-red text-sm">Error: {error}</p>
		{:else if games.length === 0}
			<div class="flex flex-col items-center gap-3 py-12 text-dark/40">
				<p class="text-lg">No games yet</p>
				<div class="w-fit duo-btn" data-duo-color="green">
					<a href="/dashboard/games/new">Create your first game</a>
				</div>
			</div>
		{:else}
			<ul class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4">
				{#each games as game, i (game.id)}
					<li
						class="relative"
						in:fly={{ y: -20, duration: 400, delay: i * 30 }}
						out:fade={{ duration: 150 }}
					>
						<PostCard
							id={game.id}
							title={game.title}
							slug={game.slug}
							excerpt={game.excerpt}
							status={game.status}
							author={{ name: game.author_name, slug: game.author_slug }}
							tags={game.tag_slugs}
							src={game.url}
							stats={game.stats}
							coverMediaType={game.cover_media_type}
							routePrefix="/games"
							dashboardPrefix="/dashboard/games/id"
						/>
						{#if game.status !== 'draft'}
							<a
								href="/dashboard/games/id/{game.id}"
								class="absolute top-2 right-2 z-20 text-base bg-dark/80 text-white px-2 py-1 rounded-lg no-underline! hover:bg-dark transition-colors"
							>
								Edit
							</a>
						{/if}
					</li>
				{/each}
			</ul>

			{#if hasMore}
				<div class="flex justify-center bg-transparent! p-0!">
					<div class="duo-btn" data-duo-color="green">
						<button onclick={fetchMore} disabled={loadingMore}>
							{loadingMore ? 'Loading…' : 'Load more'}
						</button>
					</div>
				</div>
			{/if}
		{/if}
	</div>
</section>
