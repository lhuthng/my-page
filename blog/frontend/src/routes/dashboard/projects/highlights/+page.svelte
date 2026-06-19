<script>
	import { untrack } from 'svelte';
	import { api } from '$lib/api/client';
	import { fly, fade } from 'svelte/transition';

	let { data } = $props();

	let featuredProjects = $state(untrack(() => data).featuredProjects ?? []);

	let search = $state('');
	let searchResults = $state([]);
	let searchLoading = $state(false);
	let searchError = $state(null);
	let debounceTimer;

	let featuredIds = $derived(new Set(featuredProjects.map((p) => p.id)));

	async function performSearch() {
		if (!search.trim()) {
			searchResults = [];
			return;
		}
		searchLoading = true;
		searchError = null;

		try {
			const data = await api.get('projects/all?limit=100');
			const q = search.trim().toLowerCase();
			searchResults = (data.projects ?? []).filter((p) => p.title.toLowerCase().includes(q));
		} catch (e) {
			searchError = e.message;
		} finally {
			searchLoading = false;
		}
	}

	function handleSearchInput() {
		clearTimeout(debounceTimer);
		debounceTimer = setTimeout(performSearch, 300);
	}

	async function toggleHighlight(project, currentlyFeatured) {
		const targetStatus = !currentlyFeatured;

		try {
			await api.put(`projects/id/${project.id}/featured`, {
				body: { is_featured: targetStatus }
			});

			if (targetStatus) {
				const newFeatured = {
					id: project.id,
					title: project.title,
					slug: project.slug,
					excerpt: project.excerpt,
					author_name: project.author_name,
					author_slug: project.author_slug,
					url: project.url,
					stats: project.stats || { views: 0, likes: 0, comments: 0 }
				};
				featuredProjects = [...featuredProjects, newFeatured];
			} else {
				featuredProjects = featuredProjects.filter((p) => p.id !== project.id);
			}
		} catch (e) {
			alert(`Failed to update highlight status: ${e.message}`);
		}
	}
</script>

<svelte:head>
	<title>Project Highlights - Dashboard | Huu Thang's Blog</title>
</svelte:head>

<div class="flex flex-col gap-4 pb-8">
	<div class="bg-white rounded-xl p-6 shadow-sm">
		<h1 class="text-3xl font-bold text-dark">Project Highlights</h1>
		<p class="text-base text-dark/60 mt-1">
			Select which projects are featured on the homepage. The featured section displays the 5 most
			recent featured projects.
		</p>
	</div>

	<div class="grid grid-cols-1 lg:grid-cols-12 gap-4">
		<div class="lg:col-span-7 bg-white rounded-xl p-6 shadow-sm flex flex-col gap-4 h-fit">
			<h2 class="text-2xl font-semibold text-dark flex items-center gap-2">
				Currently Featured
				<span class="text-base font-normal text-dark/40">({featuredProjects.length})</span>
			</h2>

			{#if featuredProjects.length === 0}
				<div
					class="flex flex-col items-center justify-center py-16 text-dark/40 border-2 border-dashed border-background/60 rounded-xl"
					in:fade
				>
					<p class="text-lg font-medium">No projects featured yet</p>
					<p class="text-sm text-dark/30 mt-1 text-center max-w-sm px-4">
						Use the search box on the right to find projects and highlight them on the homepage.
					</p>
				</div>
			{:else}
				<ul class="flex flex-col gap-3">
					{#each featuredProjects as project (project.id)}
						<li
							class="flex items-center gap-4 p-3.5 border border-background/50 rounded-xl hover:bg-background/5 transition-colors"
							in:fly={{ y: 20, duration: 300 }}
							out:fade={{ duration: 150 }}
						>
							{#if project.url}
								<img src={project.url} alt="" class="w-16 h-16 rounded-lg object-cover shrink-0" />
							{:else}
								<div
									class="w-16 h-16 rounded-lg bg-background/40 shrink-0 flex items-center justify-center text-dark/30 text-xs font-semibold"
								>
									No Cover
								</div>
							{/if}

							<div class="flex-1 min-w-0">
								<a
									href="/projects/{project.slug}"
									target="_blank"
									class="font-bold text-lg text-dark hover:text-primary hover:underline truncate block"
								>
									{project.title}
								</a>
								<p class="text-sm text-dark/50 mt-0.5">
									by {project.author_name} · @{project.author_slug}
								</p>
								<div class="flex gap-4 mt-2 text-xs text-dark/40">
									<span>👁 {project.stats?.views ?? 0} views</span>
									<span>❤️ {project.stats?.likes ?? 0} likes</span>
									<span>💬 {project.stats?.comments ?? 0} comments</span>
								</div>
							</div>

							<div class="shrink-0 duo-btn" data-duo-color="red">
								<button
									onclick={() => toggleHighlight(project, true)}
									class="px-3 py-1.5 text-sm font-semibold"
								>
									Remove
								</button>
							</div>
						</li>
					{/each}
				</ul>
			{/if}
		</div>

		<div class="lg:col-span-5 bg-white rounded-xl p-6 shadow-sm flex flex-col gap-4">
			<h2 class="text-2xl font-semibold text-dark">Add to Highlights</h2>

			<div
				class="relative flex items-center gap-2 bg-background/30 rounded-xl px-3 py-2.5 border border-background/60"
			>
				<span class="text-dark/40 text-lg">🔍</span>
				<input
					type="text"
					placeholder="Search published projects..."
					bind:value={search}
					oninput={handleSearchInput}
					class="flex-1 bg-transparent text-base placeholder:text-dark/30 outline-none text-dark"
				/>
				{#if search}
					<button
						onclick={() => {
							search = '';
							searchResults = [];
						}}
						class="text-dark/40 hover:text-dark text-sm cursor-pointer pr-1"
					>
						✕
					</button>
				{/if}
			</div>

			<div class="flex-1 min-h-[350px]">
				{#if searchLoading}
					<div class="flex justify-center items-center py-16 text-dark/40" in:fade>
						Searching...
					</div>
				{:else if searchError}
					<p class="text-accent-red text-sm py-4" in:fade>Error: {searchError}</p>
				{:else if search.trim() && searchResults.length === 0}
					<div class="flex flex-col items-center justify-center py-16 text-dark/40" in:fade>
						<p class="text-lg font-medium">No projects match your search</p>
					</div>
				{:else if !search.trim()}
					<div class="flex flex-col items-center justify-center py-16 text-dark/30 h-full" in:fade>
						<p class="text-center text-sm px-6">
							Type in the search box to find projects and highlight them on the homepage.
						</p>
					</div>
				{:else}
					<ul class="flex flex-col gap-3" in:fade>
						{#each searchResults as project (project.id)}
							{@const isFeatured = featuredIds.has(project.id)}
							{@const isDraft = project.status === 'draft'}

							<li
								class="flex items-center gap-3 p-3 border border-background/40 rounded-xl hover:bg-background/5 transition-colors"
							>
								{#if project.url}
									<img
										src={project.url}
										alt=""
										class="w-12 h-12 rounded-lg object-cover shrink-0"
									/>
								{:else}
									<div
										class="w-12 h-12 rounded-lg bg-background/40 shrink-0 flex items-center justify-center text-dark/30 text-xs font-semibold"
									>
										No Cover
									</div>
								{/if}

								<div class="flex-1 min-w-0">
									<span class="font-bold text-base text-dark truncate block leading-tight">
										{project.title}
									</span>
									<div class="flex items-center gap-2 mt-1 text-xs">
										<span
											class="px-1.5 py-0.5 rounded-full {isDraft
												? 'bg-accent-yellow/20 text-dark/70'
												: 'bg-accent-green/20 text-accent-green'} font-semibold"
										>
											{project.status}
										</span>
										{#if isDraft}
											<span class="text-accent-red font-medium">
												Draft (Will not display on homepage)
											</span>
										{/if}
									</div>
								</div>

								<div class="shrink-0">
									{#if isFeatured}
										<div class="duo-btn" data-duo-color="red">
											<button
												onclick={() => toggleHighlight(project, true)}
												class="px-2.5 py-1.5 text-xs font-bold"
											>
												Remove
											</button>
										</div>
									{:else}
										<div class="duo-btn" data-duo-color="red">
											<button
												onclick={() => toggleHighlight(project, false)}
												class="px-2.5 py-1.5 text-xs font-bold"
											>
												Highlight
											</button>
										</div>
									{/if}
								</div>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		</div>
	</div>
</div>
