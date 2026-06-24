<script>
	import { gql } from '$lib/api/graphql';
	import PostCard from '$lib/components/home/PostCard.svelte';
	import { onMount, untrack } from 'svelte';
	import { fly, fade } from 'svelte/transition';

	let { data } = $props();

	let projects = $state(untrack(() => data).projects ?? []);
	let total = $state(untrack(() => data).total ?? 0);
	let loading = $state(false);
	let loadingMore = $state(false);
	let error = $state(null);
	let search = $state('');
	let debounceTimer;
	const LIMIT = 9;

	function mapItem(item) {
		return {
			...item,
			url: item.coverUrl,
			cover_media_type: item.coverMediaType,
			stats: { views: item.views, likes: item.likes, comments_count: item.commentsCount }
		};
	}

	async function fetchProjects(reset = false) {
		if (reset) {
			projects = [];
			loading = true;
		} else {
			if (loadingMore) return;
			loadingMore = true;
		}
		error = null;

		try {
			const result = await gql.dashboardProjects({
				limit: LIMIT,
				offset: reset ? 0 : projects.length,
				search: search.trim() || undefined
			});
			const items = result.dashboardProjects.items.map(mapItem);
			projects = reset ? items : [...projects, ...items];
			total = result.dashboardProjects.total;
		} catch (e) {
			error = e.message;
		} finally {
			loading = false;
			loadingMore = false;
		}
	}

	function onSearchInput() {
		clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => fetchProjects(true), 400);
	}

	let hasMore = $derived(projects.length < total);

	onMount(() => fetchProjects(true));
</script>

<svelte:head>
	<title>Projects - Dashboard | Huu Thang's Blog</title>
</svelte:head>

<section class="flex flex-col gap-4 *:bg-white *:rounded-xl *:p-4 pb-8">
	<div class="flex flex-col gap-4">
		<!-- Header -->
		<div class="flex flex-wrap items-center justify-between gap-3">
			<h1 class="text-2xl font-semibold">
				Projects
				<span class="text-dark/40 text-lg font-normal">({total})</span>
			</h1>
			<div class="w-fit duo-btn" data-duo-color="green">
				<a href="/dashboard/projects/new">New Project</a>
			</div>
		</div>

		<!-- Search -->
		<div
			class="flex items-center gap-2 bg-background/40 rounded-xl px-3 py-2 border border-background"
		>
			<input
				type="text"
				placeholder="Search by title or slug…"
				bind:value={search}
				oninput={onSearchInput}
				class="flex-1 bg-transparent text-base placeholder:text-dark/30 outline-none"
			/>
			{#if search}
				<button
					onclick={() => {
						search = '';
						fetchProjects(true);
					}}
					class="text-dark/40 hover:text-dark text-sm cursor-pointer"
				>
					✕
				</button>
			{/if}
		</div>

		<!-- Content -->
		{#if loading}
			<div class="flex justify-center items-center py-12 text-dark/40">Loading…</div>
		{:else if error}
			<p class="text-accent-red text-sm">Error: {error}</p>
		{:else if projects.length === 0}
			<div class="flex flex-col items-center gap-3 py-12 text-dark/40">
				<p class="text-lg">
					{search ? 'No projects match your search' : 'No projects yet'}
				</p>
				{#if !search}
					<div class="w-fit duo-btn" data-duo-color="green">
						<a href="/dashboard/projects/new">Create your first project</a>
					</div>
				{/if}
			</div>
		{:else}
			<ul class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4">
				{#each projects as project, i (project.id)}
					<li
						class="relative"
						in:fly={{ y: -20, duration: 400, delay: i * 30 }}
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
							coverMediaType={project.cover_media_type}
							routePrefix="/projects"
							dashboardPrefix="/dashboard/projects/id"
						/>
						{#if project.status !== 'draft'}
							<a
								href="/dashboard/projects/id/{project.id}"
								class="absolute top-2 right-2 z-20 text-base bg-dark/80 text-white px-2 py-1 rounded-lg no-underline! hover:bg-dark transition-colors"
							>
								Edit
							</a>
						{/if}
					</li>
				{/each}
			</ul>

			{#if hasMore}
				<div class="flex justify-center bg-transparent! p-0!">
					<div class="duo-btn" data-duo-color="green">
						<button onclick={() => fetchProjects(false)} disabled={loadingMore}>
							{loadingMore ? 'Loading…' : `Load more (${total - projects.length} remaining)`}
						</button>
					</div>
				</div>
			{/if}
		{/if}
	</div>
</section>
