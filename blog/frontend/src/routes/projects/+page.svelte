<script>
	import PostCard from '$lib/components/home/PostCard.svelte';
	import { api } from '$lib/client/api-client';
	import { onMount } from 'svelte';
	import { flip } from 'svelte/animate';
	import { fade, fly } from 'svelte/transition';

	let { data } = $props();

	const limit = data.firstOffset ?? 5;
	const itemDelay = 45;

	let batchId = 0;

	let projects = $state(
		data.status === 'success'
			? (data.projects ?? []).map((project, index) => ({
					...project,
					_batchId: batchId,
					_introDelay: index * itemDelay
				}))
			: []
	);

	let length = $derived(projects.length);
	let hasMore = $state(Boolean(data.has_more));
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
		style:transition-duration={hydrated ? '700ms' : '0ms'}
	>
		<div class="min-h-0">
			{#if data.status !== 'success'}
				<div class="text-dark/60">Could not load projects right now.</div>
			{:else if length === 0}
				<div class="text-dark/60">No published projects yet.</div>
			{:else}
				<ul class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4">
					{#each projects as project (project.id)}
						<li
							animate:flip={{ duration: 250 }}
							class:intro={hydrated}
							style:--delay={`${project._introDelay}ms`}
						>
							<PostCard
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
						<li
							class="flex justify-center items-center full min-w-22 sm:min-w-26 min-h-22 sm:min-h-26 md:min-w-34 md:min-h-34 rounded-lg border-2 border-dashed"
							in:fly={{ y: -28, duration: 420, delay: length * itemDelay }}
							out:fade={{ duration: 150 }}
						>
							<div class="duo-btn" data-duo-color="blue">
								<button
									type="button"
									class="no-underline!"
									disabled={isLoadingMore || !hasMore}
									onclick={fetchMore}
								>
									{#if isLoadingMore}
										Loading more projects...
									{:else if hasMore}
										Load more projects
									{:else}
										No more to load
									{/if}
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
