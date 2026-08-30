<script>
	import { gql } from '$lib/api/graphql';
	import PostCard from '$lib/components/home/PostCard.svelte';
	import PageHeader from '$lib/components/dashboard/PageHeader.svelte';
	import SearchInput from '$lib/components/dashboard/SearchInput.svelte';
	import EmptyState from '$lib/components/dashboard/EmptyState.svelte';
	import LoadingCards from '$lib/components/dashboard/LoadingCards.svelte';
	import { onMount, untrack } from 'svelte';
	import { fly, fade } from 'svelte/transition';

	let { data } = $props();

	let posts = $state(untrack(() => data).posts ?? []);
	let total = $state(untrack(() => data).total ?? 0);
	let loading = $state(false);
	let loadingMore = $state(false);
	let error = $state(null);
	let search = $state('');
	let debounceTimer;
	const LIMIT = 9;

	function mapPost(item) {
		return {
			...item,
			url: item.coverUrl,
			cover_media_type: item.coverMediaType,
			stats: { views: item.views, likes: item.likes, comments_count: item.commentsCount }
		};
	}

	async function fetchPosts(reset = false) {
		if (reset) {
			posts = [];
			loading = true;
		} else {
			if (loadingMore) return;
			loadingMore = true;
		}
		error = null;

		try {
			const result = await gql.dashboardPosts({
				limit: LIMIT,
				offset: reset ? 0 : posts.length,
				search: search.trim() || undefined
			});
			const items = result.dashboardPosts.items.map(mapPost);
			posts = reset ? items : [...posts, ...items];
			total = result.dashboardPosts.total;
		} catch (e) {
			error = e.message;
		} finally {
			loading = false;
			loadingMore = false;
		}
	}

	function onSearchInput() {
		clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => fetchPosts(true), 400);
	}

	let hasMore = $derived(posts.length < total);

	onMount(() => fetchPosts(true));
</script>

<svelte:head>
	<title>Posts - Dashboard | Huu Thang's Blog</title>
</svelte:head>

<section class="flex flex-col gap-4 pb-8">
	<div class="bg-white rounded-xl p-4 flex flex-col gap-4">
		<PageHeader title="Posts" count={total}>
			{#snippet actions()}
				<div class="w-fit duo-btn" data-duo-color="green">
					<a href="/dashboard/posts/new">New Post</a>
				</div>
			{/snippet}
		</PageHeader>

		<SearchInput placeholder="Search by title or slug…" bind:value={search} onsearch={onSearchInput} onclear={() => fetchPosts(true)} />

		<!-- Content -->
		{#if loading}
			<LoadingCards grid count={3} />
		{:else if error}
			<p class="text-accent-red text-sm">Error: {error}</p>
		{:else if posts.length === 0}
			<EmptyState
				message={search ? 'No posts match your search' : 'No posts yet'}
				hint={search ? 'Try a different title or slug.' : ''}
				mascot={!search}
			>
				{#if !search}
					<div class="w-fit duo-btn" data-duo-color="green">
						<a href="/dashboard/posts/new">Create your first post</a>
					</div>
				{/if}
			</EmptyState>
		{:else}
			<ul class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4">
				{#each posts as post, i (post.id)}
					<li
						class="relative"
						in:fly={{ y: -20, duration: 400, delay: i * 30 }}
						out:fade={{ duration: 150 }}
					>
						<PostCard
							id={post.id}
							title={post.title}
							slug={post.slug}
							excerpt={post.excerpt}
							status={post.status}
							author={{ name: post.author_name, slug: post.author_slug }}
							tags={post.tag_slugs}
							src={post.url}
							stats={post.stats}
							coverMediaType={post.cover_media_type}
						/>
						<!-- Edit overlay for published posts (drafts already link to edit via PostCard) -->
						{#if post.status !== 'draft'}
							<a
								href="/dashboard/posts/id/{post.id}"
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
						<button onclick={() => fetchPosts(false)} disabled={loadingMore}>
							{loadingMore ? 'Loading…' : `Load more (${total - posts.length} remaining)`}
						</button>
					</div>
				</div>
			{/if}
		{/if}
	</div>
</section>
