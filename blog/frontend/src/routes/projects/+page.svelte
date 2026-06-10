<script>
	import PostCard from '$lib/components/home/PostCard.svelte';
	import { onMount } from 'svelte';
	import { flip } from 'svelte/animate';
	import { fade, fly } from 'svelte/transition';

	let { data } = $props();

	const limit = data.firstOffset ?? 10;
	const normalizeProject = (project) =>
		project?.url && !project.url.startsWith('/api/') && !project.url.startsWith('http')
			? { ...project, url: `/api/${project.url}`.replace('/./', '/') }
			: project;

	let projects = $state(
		data.status === 'success' ? (data.projects ?? []).map(normalizeProject) : []
	);
	let hasMore = $state(Boolean(data.has_more));
	let isLoadingMore = $state(false);
	let loadError = $state('');

	const fetchMore = async () => {
		if (isLoadingMore || !hasMore) return;

		isLoadingMore = true;
		loadError = '';

		const res = await fetch(`/api/projects/latest?limit=${limit}&offset=${projects.length}`, {
			method: 'GET'
		});

		if (!res.ok) {
			loadError = 'Could not load more projects right now.';
			isLoadingMore = false;
			return;
		}

		const payload = await res.json();
		projects = [...projects, ...(payload.projects ?? []).map(normalizeProject)];
		hasMore = Boolean(payload.has_more);
		isLoadingMore = false;
	};

	let expanded = $state(false);

	onMount(() => {
		expanded = true;
	});
</script>

<svelte:head>
	<title>Projects | Huu Thang's Blog</title>
	<meta property="og:title" content="Projects" />
	<meta name="description" content="Playable demos and project writeups." />
	<meta property="og:description" content="Playable demos and project writeups." />
	<meta property="og:type" content="website" />
</svelte:head>

<div class="bg-white rounded-xl p-4 mb-2 lg:mb-4 space-y-4">
	<h1 class="text-2xl font-semibold">Projects</h1>
	<div
		class="overflow-hidden grid transition-[grid-template-rows] duration-700 ease-out"
		style:grid-template-rows={expanded ? '1fr' : '0fr'}
	>
		<div class="min-h-0 overflow-hidden">
			{#if data.status !== 'success'}
				<div class="text-dark/60">Could not load projects right now.</div>
			{:else if projects.length === 0}
				<div class="text-dark/60">No published projects yet.</div>
			{:else}
				<ul class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4">
					{#each projects as project, index (project.id)}
						<li
							animate:flip={{ duration: 250 }}
							in:fly={{ y: -28, duration: 420, delay: index < limit ? index * 45 : 0 }}
							out:fade={{ duration: 150 }}
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
					{#if hasMore || isLoadingMore}
						<li
							class="flex justify-center items-center full min-w-22 sm:min-w-26 min-h-22 sm:min-h-26 md:min-w-34 md:min-h-34 rounded-lg border-2 border-dashed"
							in:fly={{ y: -12, duration: 300 }}
							out:fade={{ duration: 150 }}
						>
							<div class="duo-btn duo-blue">
								<button
									type="button"
									class="no-underline!"
									disabled={isLoadingMore}
									onclick={fetchMore}
								>
									{isLoadingMore ? 'Loading more posts...' : 'Load more posts'}
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
