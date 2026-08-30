<script>
	import { gql, fixUrl } from '$lib/api/graphql';
	import { api } from '$lib/api/client.js';
	import PageHeader from '$lib/components/dashboard/PageHeader.svelte';
	import SearchInput from '$lib/components/dashboard/SearchInput.svelte';
	import EmptyState from '$lib/components/dashboard/EmptyState.svelte';
	import { onMount, untrack } from 'svelte';
	import { fly, fade } from 'svelte/transition';

	let { data } = $props();

	// Active state for currently featured posts
	let featuredPosts = $state(untrack(() => data).featuredPosts ?? []);
	let loading = $state(!untrack(() => data).featuredPosts?.length);
	let actionError = $state('');

	onMount(async () => {
		if (!data.featuredPosts?.length) {
			try {
				const result = await gql.request(
					`query { featuredPosts(limit:100) { id title slug excerpt coverUrl coverMediaType authorName authorSlug views likes commentsCount } }`
				);
				featuredPosts = (result.featuredPosts ?? []).map((p) => ({
					id: p.id,
					title: p.title,
					slug: p.slug,
					excerpt: p.excerpt,
					author_name: p.authorName,
					author_slug: p.authorSlug,
					url: fixUrl(p.coverUrl),
					cover_media_type: p.coverMediaType,
					stats: { views: p.views, likes: p.likes, comments: p.commentsCount }
				}));
			} catch {
				/* ignore */
			}
			loading = false;
		}
	});

	// Search states
	let search = $state('');
	let searchResults = $state([]);
	let searchLoading = $state(false);
	let searchError = $state(null);
	let debounceTimer;

	// Track featured post IDs as a derived state for fast lookup
	let featuredIds = $derived(new Set(featuredPosts.map((p) => p.id)));

	async function performSearch() {
		if (!search.trim()) {
			searchResults = [];
			return;
		}
		searchLoading = true;
		searchError = null;

		try {
			const result = await gql.dashboardPosts({ limit: 10, search: search.trim() });
			searchResults = (result.dashboardPosts?.items ?? []).map((p) => ({
				...p,
				author_name: p.authorName,
				author_slug: p.authorSlug,
				url: p.coverUrl,
				cover_media_type: p.coverMediaType,
				stats: { views: p.views, likes: p.likes, comments_count: p.commentsCount }
			}));
		} catch (e) {
			searchError = e.message;
		} finally {
			searchLoading = false;
		}
	}

	function handleSearchInput() {
		clearTimeout(debounceTimer);
		debounceTimer = setTimeout(performSearch, 300);
	}

	async function toggleHighlight(post, currentlyFeatured) {
		const targetStatus = !currentlyFeatured;

		try {
			await api.put(`posts/id/${post.id}/featured`, {
				body: { is_featured: targetStatus }
			});

			// Update local state reactively
			if (targetStatus) {
				const newFeatured = {
					id: post.id,
					title: post.title,
					slug: post.slug,
					excerpt: post.excerpt,
					author_name: post.author_name,
					author_slug: post.author_slug,
					url: post.url,
					cover_media_type: post.cover_media_type,
					stats: post.stats || { views: 0, likes: 0, comments: 0 }
				};
				featuredPosts = [...featuredPosts, newFeatured];
			} else {
				featuredPosts = featuredPosts.filter((p) => p.id !== post.id);
			}
		} catch (e) {
			actionError = `Failed to update highlight status: ${e.message}`;
		}
	}
</script>

<svelte:head>
	<title>Highlight Posts - Dashboard | Huu Thang's Blog</title>
</svelte:head>

<div class="flex flex-col gap-4 pb-8">
	<!-- Header Card -->
	<div class="bg-white rounded-xl p-4">
		<h1 class="text-2xl font-semibold text-dark">Highlight Posts</h1>
		<p class="text-base text-dark/60 mt-1">
			Select which posts are featured in the "Discover" tab on the homepage. The discover section on
			the homepage displays the 5 most recent featured posts.
		</p>
	</div>

	<div class="grid grid-cols-1 lg:grid-cols-12 gap-4">
		<!-- Currently Featured Column (Left) -->
		<div class="lg:col-span-7 bg-white rounded-xl p-4 flex flex-col gap-4 h-fit">
			<h2 class="text-2xl font-semibold text-dark flex items-center gap-2">
				Currently Featured
				<span class="text-base font-normal text-dark/40">({featuredPosts.length})</span>
			</h2>

			{#if featuredPosts.length === 0}
				<div
					class="flex flex-col items-center justify-center py-16 text-dark/40 border-2 border-dashed border-background/60 rounded-xl"
					in:fade
				>
					<p class="text-lg font-medium">No posts featured yet</p>
					<p class="text-sm text-dark/30 mt-1 text-center max-w-sm px-4">
						Use the search box on the right to find posts and highlight them on the homepage.
					</p>
				</div>
			{:else}
				<ul class="flex flex-col gap-3">
					{#each featuredPosts as post (post.id)}
						<li
							class="flex items-center gap-4 p-3.5 border-2 border-dark/10 rounded-xl hover:bg-background/40 transition-colors"
							in:fly={{ y: 20, duration: 300 }}
							out:fade={{ duration: 150 }}
						>
							{#if post.url}
								{#if post.cover_media_type?.startsWith('video/')}
									<video
										src={post.url}
										poster={post.url}
										class="w-16 h-16 rounded-lg object-cover shrink-0"
										muted
										loop
										playsinline
										autoplay
										preload="auto"
									></video>
								{:else}
									<img src={post.url} alt="" class="w-16 h-16 rounded-lg object-cover shrink-0" />
								{/if}
							{:else}
								<div
									class="w-16 h-16 rounded-lg bg-background/40 shrink-0 flex items-center justify-center text-dark/30 text-xs font-semibold"
								>
									No Cover
								</div>
							{/if}

							<div class="flex-1 min-w-0">
								<a
									href="/posts/{post.slug}"
									target="_blank"
									class="font-bold text-lg text-dark hover:text-primary hover:underline truncate block"
								>
									{post.title}
								</a>
								<p class="text-sm text-dark/50 mt-0.5">
									by {post.author_name} · @{post.author_slug}
								</p>
								<div class="flex gap-4 mt-2 text-xs text-dark/40">
									<span>👁 {post.stats?.views ?? 0} views</span>
									<span>❤️ {post.stats?.likes ?? 0} likes</span>
									<span>💬 {post.stats?.comments ?? 0} comments</span>
								</div>
							</div>

							<div class="duo-btn shrink-0" data-duo-color="red">
								<button
									onclick={() => toggleHighlight(post, true)}
									class="px-3 py-1.5 text-sm font-semibold"
								>
									Remove
								</button>
							</div>
						</li>
					{/each}
				</ul>
			{/if}
		</div>

		<!-- Add to Highlights Column (Right) -->
		<div class="lg:col-span-5 bg-white rounded-xl p-4 flex flex-col gap-4">
			<h2 class="text-2xl font-semibold text-dark">Add to Highlight Posts</h2>

			<!-- Search input field -->
			<SearchInput placeholder="Search published posts..." bind:value={search} onsearch={handleSearchInput} onclear={() => (searchResults = [])} />

			{#if actionError}
				<p class="text-accent-red text-sm">Error: {actionError}</p>
			{/if}
			{#if searchLoading}
				<div class="flex justify-center items-center py-16 text-dark/40" in:fade>
					Searching...
				</div>
			{:else if searchError}
				<p class="text-accent-red text-sm py-4" in:fade>Error: {searchError}</p>
			{:else if search.trim() && searchResults.length === 0}
				<div class="flex flex-col items-center justify-center py-16 text-dark/40" in:fade>
					<p class="text-lg font-medium">No posts match your search</p>
				</div>
			{:else if !search.trim()}
				<EmptyState
					message="Find posts to highlight"
					hint="Type in the search box to find posts and highlight/recommend them on the homepage."
				/>
			{:else}
					<ul class="flex flex-col gap-3" in:fade>
						{#each searchResults as post (post.id)}
							{@const isFeatured = featuredIds.has(post.id)}
							{@const isDraft = post.status === 'draft'}

							<li
								class="flex items-center gap-3 p-3 border-2 border-dark/10 rounded-xl hover:bg-background/40 transition-colors"
							>
								{#if post.url}
									{#if post.cover_media_type?.startsWith('video/')}
										<video
											src={post.url}
											poster={post.url}
											class="w-12 h-12 rounded-lg object-cover shrink-0"
											muted
											loop
											playsinline
											autoplay
											preload="auto"
										></video>
									{:else}
										<img src={post.url} alt="" class="w-12 h-12 rounded-lg object-cover shrink-0" />
									{/if}
								{:else}
									<div
										class="w-12 h-12 rounded-lg bg-background/40 shrink-0 flex items-center justify-center text-dark/30 text-xs font-semibold"
									>
										No Cover
									</div>
								{/if}

								<div class="flex-1 min-w-0">
									<span class="font-bold text-base text-dark truncate block leading-tight">
										{post.title}
									</span>
									<div class="flex items-center gap-2 mt-1 text-xs">
										<span
											class="px-1.5 py-0.5 rounded-full {isDraft
												? 'bg-accent-yellow/20 text-dark/70'
												: 'bg-accent-green/20 text-accent-green'} font-semibold"
										>
											{post.status}
										</span>
										{#if isDraft}
											<span class="text-accent-red font-medium">
												Draft (Will not display on homepage)
											</span>
										{/if}
									</div>
								</div>

								<div class="shrink-0">
									{#if isFeatured}
										<div class="duo-btn" data-duo-color="red">
											<button
												onclick={() => toggleHighlight(post, true)}
												class="px-2.5 py-1.5 text-xs font-bold"
											>
												Remove
											</button>
										</div>
									{:else}
										<div class="duo-btn" data-duo-color="green">
											<button
												onclick={() => toggleHighlight(post, false)}
												class="px-2.5 py-1.5 text-xs font-bold"
											>
												Highlight
											</button>
										</div>
									{/if}
								</div>
							</li>
						{/each}
					</ul>
				{/if}
		</div>
	</div>
</div>
