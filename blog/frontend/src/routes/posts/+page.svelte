<script>
	import PostCard from '$lib/components/home/PostCard.svelte';
	import { api } from '$lib/api/client';
	import { onMount, untrack } from 'svelte';
	import { flip } from 'svelte/animate';
	import ExploreMore from '$lib/components/home/ExploreMore.svelte';
	import FetchMore from '$lib/components/home/FetchMore.svelte';
	import GridExpander from '$lib/components/shell/GridExpander.svelte';
	import BackButton from '$lib/components/ui/BackButton.svelte';
	import { SITE_OG_IMAGE } from '$lib/config/site.js';

	let { data } = $props();

	const limit = $derived(untrack(() => data.firstOffset ?? 5));
	const itemDelay = 45;
	const imageUrl = SITE_OG_IMAGE;

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
	<title>Posts | Huu Thang's Blog</title>
	<meta property="og:title" content="Posts | Huu Thang's Blog" />
	<meta name="description" content="Recent posts, essays, and updates." />
	<meta property="og:description" content="Recent posts, essays, and updates." />
	<meta property="og:type" content="website" />
	<meta property="og:image" content={imageUrl} />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content="Posts | Huu Thang's Blog" />
	<meta name="twitter:description" content="Recent posts, essays, and updates." />
	<meta name="twitter:image" content={imageUrl} />
</svelte:head>

<div class="bg-white rounded-xl mb-2 lg:mb-4">
	<div class="px-4 pt-4 space-y-2">
		<BackButton href="/" text="Home" />
		<h1 class="text-2xl font-semibold">Posts</h1>
	</div>

	<GridExpander
		class="p-4"
		expanded={(hydrated && expanded) || !hydrated}
		duration={hydrated ? '1s' : '0ms'}
	>
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
							readingTime={post.reading_time_minutes}
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
	</GridExpander>
</div>
