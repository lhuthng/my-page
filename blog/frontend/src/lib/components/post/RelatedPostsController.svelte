<script>
	import { api } from '$lib/api/client';
	import { useDebounce } from '$lib/utils/debounce';

	let { postId = null, relatedPosts = $bindable([]) } = $props();

	let searchTerm = $state('');
	let searchResults = $state([]);
	let searching = $state(false);
	let saving = $state(false);
	let saveError = $state('');

	const selectedSlugs = $derived(new Set(relatedPosts.map((p) => p.slug)));

	const searchDebounce = useDebounce(async (term) => {
		if (!term || term.length < 2) {
			searchResults = [];
			return;
		}
		searching = true;
		try {
			const res = await fetch(`/api/posts?term=${encodeURIComponent(term)}&size=5`);
			if (res.ok) {
				const { posts } = await res.json();
				searchResults = (posts ?? []).filter((p) => !selectedSlugs.has(p.slug));
			}
		} finally {
			searching = false;
		}
	}, 300);

	$effect(() => {
		searchDebounce.update(searchTerm);
	});

	const addPost = async (post) => {
		if (selectedSlugs.has(post.slug)) return;
		const next = [...relatedPosts, post];
		relatedPosts = next;
		searchTerm = '';
		searchResults = [];
		if (postId) await persist(next);
	};

	const removePost = async (slug) => {
		const next = relatedPosts.filter((p) => p.slug !== slug);
		relatedPosts = next;
		if (postId) await persist(next);
	};

	const persist = async (posts) => {
		if (!postId) return;
		saving = true;
		saveError = '';
		try {
			await api.patch(`posts/id/${postId}/related`, {
				body: { related_post_slugs: posts.map((p) => p.slug) }
			});
		} catch (e) {
			saveError = e.message;
		} finally {
			saving = false;
		}
	};
</script>

<div class="flex flex-col gap-2">
	<label class="inline-block font-medium" for="related-post-search">Related Posts:</label>

	{#if relatedPosts.length > 0}
		<div class="flex flex-wrap gap-2">
			{#each relatedPosts as post (post.slug)}
				<span
					class="flex items-center gap-1 px-2 py-1 rounded-full bg-primary/20 border border-primary/40 text-sm"
				>
					{post.title}
					<button
						type="button"
						class="ml-1 text-dark/60 hover:text-dark"
						onclick={() => removePost(post.slug)}
						title="Remove"
					>
						✕
					</button>
				</span>
			{/each}
		</div>
	{/if}

	<div class="relative">
		<input
			id="related-post-search"
			type="text"
			class="w-full p-1 outline-none bg-white rounded-sm border border-dark/20"
			placeholder="Search posts to link..."
			bind:value={searchTerm}
		/>
		{#if searching}
			<span class="absolute right-2 top-1/2 -translate-y-1/2 text-xs text-dark/50">searching…</span>
		{/if}
		{#if searchResults.length > 0}
			<ul
				class="absolute left-0 right-0 top-full z-20 mt-1 rounded-lg border border-dark/20 bg-white shadow-lg overflow-hidden"
			>
				{#each searchResults as post (post.slug)}
					<li>
						<button
							type="button"
							class="flex w-full items-center gap-2 px-3 py-2 text-left hover:bg-primary/10"
							onclick={() => addPost(post)}
						>
							{#if post.cover_url}
								<img
									class="w-8 h-8 object-cover rounded shrink-0"
									src={post.cover_url}
									alt={post.title}
								/>
							{/if}
							<span class="text-sm truncate">{post.title}</span>
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	</div>

	{#if saving}
		<span class="text-xs text-dark/50">Saving…</span>
	{/if}
	{#if saveError}
		<span class="text-xs text-red-500">{saveError}</span>
	{/if}
</div>
