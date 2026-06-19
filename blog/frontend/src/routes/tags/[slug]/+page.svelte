<script>
	import { untrack } from 'svelte';
	import PostCard from '$lib/components/home/PostCard.svelte';
	import { fade, fly } from 'svelte/transition';

	const { data } = $props();

	const tag = untrack(() => data).tag;
	const posts = untrack(() => data).posts ?? [];
	const projects = untrack(() => data).projects ?? [];

	const postLabel = $derived(tag.post_count === 1 ? 'item' : 'items');
</script>

<svelte:head>
	<title>#{tag.slug} | Tags | Huu Thang's Blog</title>
	<meta name="description" content={`Published posts filed under the ${tag.slug} tag.`} />
</svelte:head>

<section class="bg-white space-y-4 rounded-xl p-4 mb-2 lg:mb-4">
	<div class="flex not-md:flex-col justify-between gap-4">
		<div class="space-y-2">
			<a class="font-semibold text-accent-blue" href="/tags">All tags</a>
			<h1 class="text-2xl font-semibold">#{tag.slug}</h1>
			<p class="text-dark/70">
				{tag.post_count} published {postLabel} filed under this tag.
			</p>
			<p class="text-sm text-dark/40">
				Display name: {tag.name}
			</p>
		</div>

		<div class="h-fit rounded-xl bg-background/30 px-4 py-3">
			<div class="text-sm uppercase tracking-[0.15em] text-dark/50">Slug</div>
			<div class="font-semibold text-dark whitespace-nowrap">/{tag.slug}</div>
		</div>
	</div>

	{#if posts.length === 0 && projects.length === 0}
		<div class="rounded-xl bg-background/30 px-4 py-6 text-dark/60">
			No published posts or projects carry this tag yet.
		</div>
	{:else}
		<ul class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4">
			{#each posts as { title, slug, excerpt, author_name, author_slug, tag_slugs, url, stats, cover_media_type }, index (slug)}
				<li in:fly={{ y: -20, duration: 500 }} out:fade={{ duration: 150 }}>
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
						coverMediaType={cover_media_type}
					/>
				</li>
			{/each}
			{#each projects as { id, title, slug, excerpt, author_name, author_slug, tag_slugs, url, stats, status, cover_media_type }, index (slug)}
				<li in:fly={{ y: -20, duration: 500 }} out:fade={{ duration: 150 }}>
					<PostCard
						{id}
						{title}
						{slug}
						{excerpt}
						{status}
						author={{
							name: author_name,
							slug: author_slug
						}}
						tags={tag_slugs}
						src={url}
						{stats}
						routePrefix="/projects"
						dashboardPrefix="/dashboard/projects/id"
						coverMediaType={cover_media_type}
					/>
				</li>
			{/each}
		</ul>
	{/if}
</section>
