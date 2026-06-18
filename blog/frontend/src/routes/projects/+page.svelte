<script>
	import { api } from '$lib/api/client';
	import { onMount, untrack } from 'svelte';
	import { flip } from 'svelte/animate';
	import FetchMore from '$lib/components/home/FetchMore.svelte';
	import BigPostCard from '$lib/components/home/BigPostCard.svelte';

	let { data } = $props();

	const limit = $derived(untrack(() => data.firstOffset ?? 5));
	const itemDelay = 45;

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
	<meta property="og:title" content="Projects" />
	<meta name="description" content="Playable demos and project writeups." />
	<meta property="og:description" content="Playable demos and project writeups." />
	<meta property="og:type" content="website" />
</svelte:head>

<div class="bg-white rounded-xl mb-2 lg:mb-4">
	<h1 class="text-2xl px-4 pt-4 font-semibold">Projects</h1>

	<div
		class="grid overflow-hidden p-4"
		class:transition-[grid-template-rows]={hydrated}
		class:ease-out={hydrated}
		style:grid-template-rows={hydrated ? (expanded ? '1fr' : '0fr') : '1fr'}
		style:transition-duration={hydrated ? '1s' : '0ms'}
	>
		<div class="min-h-0">
			{#if data.status !== 'success'}
				<div class="text-dark/60">Could not load projects right now.</div>
			{:else if length === 0}
				<div class="text-dark/60">No published projects yet.</div>
			{:else}
				<ul class="grid grid-cols-[repeat(auto-fill,minmax(15rem,1fr))] gap-4">
					{#each projects as project (project.id)}
						<li
							animate:flip={{ duration: 250 }}
							class:intro={hydrated}
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
								routePrefix="/projects"
								dashboardPrefix="/dashboard/projects/id"
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
		</div>
	</div>
</div>

<style>
	.intro {
		animation: fly-in 420ms ease-out both;
		animation-delay: var(--delay);
	}

	@keyframes fly-in {
		from {
			opacity: 0;
			transform: translateY(-28px);
		}

		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
</style>
