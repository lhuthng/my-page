<script>
	import BackButton from '$lib/components/ui/BackButton.svelte';
	import { untrack } from 'svelte';
	import { SITE_OG_IMAGE } from '$lib/config/site.js';

	const { data } = $props();

	const tags = untrack(() => data).tags ?? [];
	const term = untrack(() => data).term ?? '';
	const imageUrl = SITE_OG_IMAGE;

	const tagLabel = $derived(tags.length === 1 ? 'tag' : 'tags');
</script>

<svelte:head>
	<title>Tags | Huu Thang's Blog</title>
	<meta
		name="description"
		content="Browse the recurring topics, experiments, and side quests across the blog."
	/>
	<meta property="og:title" content="Tags | Huu Thang's Blog" />
	<meta
		property="og:description"
		content="Browse the recurring topics, experiments, and side quests across the blog."
	/>
	<meta property="og:type" content="website" />
	<meta property="og:image" content={imageUrl} />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content="Tags | Huu Thang's Blog" />
	<meta
		name="twitter:description"
		content="Browse the recurring topics, experiments, and side quests across the blog."
	/>
	<meta name="twitter:image" content={imageUrl} />
</svelte:head>

<section class="bg-white rounded-xl p-4 mb-2 lg:mb-4 space-y-4">
	<div class="flex not-md:flex-col justify-between gap-4">
		<div class="space-y-2">
			<BackButton href="/" text="Home" />
			<h1 class="text-2xl font-semibold">Tags</h1>
			<p class="max-w-2xl text-dark/70">
				Browse the recurring topics, rabbit holes, and experiments that keep showing up across the
				blog.
			</p>
		</div>
	</div>

	<div class="flex not-sm:flex-col not-sm:items-start items-center justify-between gap-2">
		<p class="text-dark/70">
			{#if term}
				Showing {tags.length}
				{tagLabel} matching slug
				<span class="font-semibold text-dark">#{term}</span>
			{:else}
				Browse {tags.length} published {tagLabel}.
			{/if}
		</p>
	</div>

	{#if tags.length === 0}
		<div class="rounded-xl bg-background/30 px-4 py-6 text-dark/60">
			No published tags match that search yet.
		</div>
	{:else}
		<ul class="flex flex-wrap gap-3">
			{#each tags as { name, slug, post_count } (slug)}
				<li>
					<a
						class="flex items-center gap-2 rounded-full bg-background/40 px-3 py-2 transition-colors duration-100 hover:bg-background/60"
						href={`/tags/${slug}`}
					>
						<span class="font-semibold text-dark">#{slug}</span>
						<span class="text-sm text-dark/40">{name}</span>
						<span class="rounded-full bg-white px-2 py-0.5 text-sm text-dark/60">
							{post_count}
						</span>
					</a>
				</li>
			{/each}
		</ul>
	{/if}
</section>
