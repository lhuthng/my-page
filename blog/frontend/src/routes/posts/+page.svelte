<script>
	import PostCard from '$lib/components/home/PostCard.svelte';
	import { api } from '$lib/api/client';
	import { onMount, untrack } from 'svelte';
	import { flip } from 'svelte/animate';
	import ExploreMore from '$lib/components/home/ExploreMore.svelte';
	import FetchMore from '$lib/components/home/FetchMore.svelte';

	let { data } = $props();

	const limit = $derived(untrack(() => data.firstOffset ?? 5));
	const itemDelay = 45;

	let batchId = 0;

	let posts = $state(
		untrack(() =>
			data.status === 'success'
				? (data.featured_posts ?? []).map((post, index) => ({
						...post,
						_batchId: batchId,
						_introDelay: index * itemDelay
					}))
				: []
		)
	);

	let length = $derived(posts.length);
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
			const payload = await api.get(`posts/latest?limit=${limit}&offset=${length}`, {
				auth: false
			});

			batchId += 1;

			const newPosts = (payload.featured_posts ?? []).map((post, index) => ({
				...post,
				_batchId: batchId,
				_introDelay: index * itemDelay
			}));

			posts = [...posts, ...newPosts];
			hasMore = Boolean(payload.has_more);
		} catch {
			loadError = 'Could not load more posts right now.';
		} finally {
			isLoadingMore = false;
		}
	};
</script>

<svelte:head>
	<title>Posts</title>
	<meta property="og:title" content="Posts" />
	<meta name="description" content="Recent posts, essays, and updates." />
	<meta property="og:description" content="Recent posts, essays, and updates." />
	<meta property="og:type" content="website" />
</svelte:head>

<div class="bg-white rounded-xl mb-2 lg:mb-4">
	<h1 class="text-2xl px-4 pt-4 font-semibold">Posts</h1>

	<div
		class="grid px-4 p-4 overflow-hidden"
		class:transition-[grid-template-rows]={hydrated}
		class:ease-out={hydrated}
		style:grid-template-rows={hydrated ? (expanded ? '1fr' : '0fr') : '1fr'}
		style:transition-duration={hydrated ? `1s` : '0ms'}
	>
		<div class="min-h-0">
			{#if data.status !== 'success'}
				<p class="text-dark/60">Could not load posts right now.</p>
			{:else if length === 0}
				<p class="text-dark/60">No posts published yet.</p>
			{:else}
				<ul
					class="grid grid-cols-1 [&>li]:opacity-0 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4"
				>
					{#each posts as post (post.slug)}
						<li
							animate:flip={{ duration: 250 }}
							class:animate-fly-in={hydrated}
							style:--delay={`${post._introDelay}ms`}
						>
							<PostCard
								title={post.title}
								slug={post.slug}
								excerpt={post.excerpt}
								author={{
									name: post.author_name,
									slug: post.author_slug
								}}
								tags={post.tag_slugs}
								src={post.url}
								stats={post.stats}
								coverMediaType={post.cover_media_type}
							/>
						</li>
					{/each}

					{#if expanded}
						<FetchMore
							{isLoadingMore}
							{hasMore}
							label="post"
							intro={hydrated}
							delay={length * itemDelay}
							onclick={fetchMore}
						/>
					{/if}
				</ul>
			{/if}
		</div>
	</div>
</div>
