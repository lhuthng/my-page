<script>
	import { onMount } from 'svelte';
	import BigPostCard from './BigPostCard.svelte';
	import ExploreMore from './ExploreMore.svelte';

	const { featuredProjects } = $props();

	const itemDelay = 45;
	let hydrated = $state(false);

	onMount(() => {
		hydrated = true;
	});
</script>

<div class="space-y-4 pt-4">
	<h2 class="text-xl sm:text-2xl font-bold text-dark">Featured Projects</h2>

	<ul class="[&>li]:opacity-0 grid grid-cols-[repeat(auto-fill,minmax(15rem,1fr))] gap-4">
		{#each featuredProjects as { title, slug, excerpt, author_name, author_slug, tag_slugs, url, stats, cover_media_type, reading_time_minutes }, index (slug)}
			<li class:animate-fly-in={hydrated} style:--delay={`${index * itemDelay}ms`}>
				<BigPostCard
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
					readingTime={reading_time_minutes}
					routePrefix="/projects"
					coverMediaType={cover_media_type}
				/>
			</li>
		{/each}

		<ExploreMore intro={hydrated} delay={featuredProjects.length * itemDelay} href="/projects" />
	</ul>
</div>
