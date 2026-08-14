<script>
	import { api } from '$lib/api/client';
	import { onMount, untrack } from 'svelte';
	import { flip } from 'svelte/animate';
	import FetchMore from '$lib/components/home/FetchMore.svelte';
	import BigPostCard from '$lib/components/home/BigPostCard.svelte';
	import GridExpander from '$lib/components/shell/GridExpander.svelte';
	import BackButton from '$lib/components/ui/BackButton.svelte';
	import { SITE_OG_IMAGE } from '$lib/config/site.js';

	let { data } = $props();

	const limit = $derived(untrack(() => data.firstOffset ?? 5));
	const itemDelay = 45;
	const imageUrl = SITE_OG_IMAGE;

	let batchId = 0;

	let projects = $state(
		untrack(() =>
			data.status === 'success'
				? (data.projects ?? []).map((project, index) => ({
						...project,
						_batchId: batchId,
						_introDelay: index * itemDelay
					}))
				: []
		)
	);

	let length = $derived(projects.length);
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
			const payload = await api.get(`projects/latest?limit=${limit}&offset=${projects.length}`, {
				auth: false
			});

			batchId += 1;

			const newProjects = (payload.projects ?? []).map((project, index) => ({
				...project,
				_batchId: batchId,
				_introDelay: index * itemDelay
			}));

			projects = [...projects, ...newProjects];
			hasMore = Boolean(payload.has_more);
		} catch {
			loadError = 'Could not load more projects right now.';
		} finally {
			isLoadingMore = false;
		}
	};
</script>

<svelte:head>
	<title>Projects | Huu Thang's Blog</title>
	<meta property="og:title" content="Projects | Huu Thang's Blog" />
	<meta name="description" content="Playable demos and project writeups." />
	<meta property="og:description" content="Playable demos and project writeups." />
	<meta property="og:type" content="website" />
	<meta property="og:image" content={imageUrl} />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content="Projects | Huu Thang's Blog" />
	<meta name="twitter:description" content="Playable demos and project writeups." />
	<meta name="twitter:image" content={imageUrl} />
</svelte:head>

<div class="bg-white rounded-xl mb-2 lg:mb-4">
	<div class="px-4 pt-4 space-y-2">
		<BackButton href="/" text="Home" />
		<h1 class="text-2xl font-semibold">Projects</h1>
	</div>
	<GridExpander
		class="p-4"
		expanded={(hydrated && expanded) || !hydrated}
		duration={hydrated ? '1s' : '0ms'}
	>
		{#if data.status !== 'success'}
			<div class="text-dark/60">Could not load projects right now.</div>
		{:else if length === 0}
			<div class="text-dark/60">No published projects yet.</div>
		{:else}
			<ul class="grid grid-cols-[repeat(auto-fill,minmax(15rem,1fr))] gap-4">
				{#each projects as project (project.id)}
					<li
						animate:flip={{ duration: 250 }}
						class:animate-fly-in={hydrated}
						style:--delay={`${project._introDelay}ms`}
					>
						<BigPostCard
							id={project.id}
							title={project.title}
							slug={project.slug}
							excerpt={project.excerpt}
							status={project.status}
							author={{ name: project.author_name, slug: project.author_slug }}
							tags={project.tag_slugs}
							src={project.url}
							stats={project.stats}
							readingTime={project.reading_time_minutes}
							routePrefix="/projects"
							dashboardPrefix="/dashboard/projects/id"
							coverMediaType={project.cover_media_type}
						/>
					</li>
				{/each}

				{#if expanded}
					<FetchMore
						{isLoadingMore}
						{hasMore}
						label="project"
						intro={hydrated}
						delay={length * itemDelay}
						onclick={fetchMore}
					/>
				{/if}
			</ul>
		{/if}

		{#if loadError}
			<p class="text-sm text-dark/60">{loadError}</p>
		{/if}
	</GridExpander>
</div>
