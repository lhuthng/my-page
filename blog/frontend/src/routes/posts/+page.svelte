<script>
	import PostCard from '$lib/components/home/PostCard.svelte';
	import { onMount } from 'svelte';
	import { flip } from 'svelte/animate';
	import { fade, fly } from 'svelte/transition';

	let { data } = $props();

	const limit = data.firstOffset ?? 5;

	let posts = $state(data.status === 'success' ? [...(data.featured_posts ?? [])] : []);
	let hasMore = $state(Boolean(data.has_more));
	let isLoadingMore = $state(false);
	let loadError = $state('');

	const fetchMore = async () => {
		if (isLoadingMore || !hasMore) return;

		isLoadingMore = true;
		loadError = '';

		const res = await fetch(`/api/posts/latest?limit=${limit}&offset=${posts.length}`, {
			method: 'GET'
		});

		if (!res.ok) {
			loadError = 'Could not load more posts right now.';
			isLoadingMore = false;
			return;
		}

		const payload = await res.json();
		posts = [...posts, ...(payload.featured_posts ?? [])];
		hasMore = Boolean(payload.has_more);
		isLoadingMore = false;
	};

	let expanded = $state(false);

	onMount(() => {
		expanded = true;
	});
</script>

<svelte:head>
	<title>Posts</title>
	<meta property="og:title" content="Posts" />
	<meta name="description" content="Recent posts, essays, and updates." />
	<meta property="og:description" content="Recent posts, essays, and updates." />
	<meta property="og:type" content="website" />
</svelte:head>

<div class="bg-white rounded-xl p-4 mb-2 lg:mb-4 space-y-4">
	<h1 class="text-2xl font-semibold">Posts</h1>
	<div
		class="overflow-hidden grid transition-[grid-template-rows] ease-out"
		style:grid-template-rows={expanded ? '1fr' : '0fr'}
		style:transition-duration={(data?.featured_posts?.length * 100 || 0) + 'ms'}
	>
		<div class="min-h-0 overflow-hidden">
			{#if data.status !== 'success'}
				<p class="text-dark/60">Could not load posts right now.</p>
			{:else if posts.length === 0}
				<p class="text-dark/60">No posts published yet.</p>
			{:else}
				<ul class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4">
					{#each posts as { title, slug, excerpt, author_name, author_slug, tag_slugs, url, stats }, index (slug)}
						<li
							animate:flip={{ duration: 250 }}
							in:fly={{ y: -20, duration: 500, delay: index < limit ? index * 100 : 0 }}
							out:fade={{ duration: 150 }}
						>
							<PostCard
								{title}
								{slug}
								{excerpt}
								author={{
									name: author_name,
									slug: author_slug
								}}
								tags={tag_slugs}
								src={url}
								{stats}
							/>
						</li>
					{/each}
					{#if hasMore || isLoadingMore}
						<li
							class="flex justify-center items-center full min-w-22 sm:min-w-26 min-h-22 sm:min-h-26 md:min-w-34 md:min-h-34 rounded-lg border-2 border-dashed"
							in:fly={{ y: -12, duration: 300 }}
							out:fade={{ duration: 150 }}
						>
							<div class="duo-btn duo-blue">
								<button
									type="button"
									class="no-underline!"
									disabled={isLoadingMore}
									onclick={fetchMore}
								>
									{isLoadingMore ? 'Loading more posts...' : 'Load more posts'}
								</button>
							</div>
						</li>
					{/if}
				</ul>
			{/if}
			{#if loadError}
				<p class="text-sm text-dark/60">{loadError}</p>
			{/if}
		</div>
	</div>
</div>
