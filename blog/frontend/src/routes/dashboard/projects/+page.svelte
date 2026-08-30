<script>
	import { gql } from '$lib/api/graphql';
	import PostCard from '$lib/components/home/PostCard.svelte';
	import PageHeader from '$lib/components/dashboard/PageHeader.svelte';
	import SearchInput from '$lib/components/dashboard/SearchInput.svelte';
	import EmptyState from '$lib/components/dashboard/EmptyState.svelte';
	import LoadingCards from '$lib/components/dashboard/LoadingCards.svelte';
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

<section class="flex flex-col gap-4 pb-8">
	<div class="bg-white rounded-xl p-4 flex flex-col gap-4">
		<PageHeader title="Projects" count={total}>
			{#snippet actions()}
				<div class="w-fit duo-btn" data-duo-color="green">
					<a href="/dashboard/projects/new">New Project</a>
				</div>
			{/snippet}
		</PageHeader>

		<SearchInput placeholder="Search by title or slug…" bind:value={search} onsearch={onSearchInput} onclear={() => fetchProjects(true)} />

		<!-- Content -->
		{#if loading}
			<LoadingCards grid count={3} />
		{:else if error}
			<p class="text-accent-red text-sm">Error: {error}</p>
		{:else if projects.length === 0}
			<EmptyState
				message={search ? 'No projects match your search' : 'No projects yet'}
				hint={search ? 'Try a different title or slug.' : ''}
				mascot={!search}
			>
				{#if !search}
					<div class="w-fit duo-btn" data-duo-color="green">
						<a href="/dashboard/projects/new">Create your first project</a>
					</div>
				{/if}
			</EmptyState>
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
