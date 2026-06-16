<script>
	import { onMount } from 'svelte';
	import BigPostCard from './BigPostCard.svelte';

	const { featuredProjects } = $props();

	const itemDelay = 45;
	let hydrated = $state(false);

	onMount(() => {
		hydrated = true;
	});
</script>

<div class="space-y-4 pt-4">
	<h2 class="text-xl sm:text-2xl font-bold text-dark">Featured Projects</h2>

	<ul class="grid grid-cols-[repeat(auto-fill,minmax(15rem,1fr))] gap-4">
		{#each featuredProjects as { title, slug, excerpt, author_name, author_slug, tag_slugs, url, stats }, index (slug)}
			<li class:intro={hydrated} style:--delay={`${index * itemDelay}ms`}>
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
				/>
			</li>
		{/each}

		<li
			class="flex justify-center items-center full min-w-22 sm:min-w-26 min-h-22 sm:min-h-26 md:min-w-34 md:min-h-34 rounded-lg border-2 border-dashed"
			class:intro={hydrated}
			style:--delay={`${featuredProjects.length * itemDelay}ms`}
		>
			<div class="duo-btn duo-blue">
				<a class="no-underline!" href="/projects">explore more</a>
			</div>
		</li>
	</ul>
</div>

<style lang="postcss">
	@reference "../../../app.css";

	.intro {
		animation: fly-in 420ms ease-out both;
		animation-delay: var(--delay);
	}

	@keyframes fly-in {
		from {
			opacity: 0;
			transform: translateY(-20px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
</style>
