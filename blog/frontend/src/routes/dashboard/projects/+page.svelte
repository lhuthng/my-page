<script>
	import PostCard from '$lib/components/home/PostCard.svelte';

	let { data } = $props();
	const projects = $derived(data.projects ?? []);
</script>

<section class="flex flex-col gap-4 pb-4">
	<div class="flex items-center justify-between bg-white rounded-xl p-4 drop-shadow-xl">
		<h1 class="text-3xl">Projects</h1>
		<div class="duo-btn" data-duo-color="green">
			<a href="/dashboard/projects/new">New project</a>
		</div>
	</div>

	{#if projects.length === 0}
		<div class="bg-white rounded-xl p-4 text-dark/60">No projects yet.</div>
	{:else}
		<ul class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4">
			{#each projects as project (project.id)}
				<li class="relative">
					<PostCard
						id={project.id}
						dashboardMode={true}
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
					<a
						class="absolute inset-0 z-20"
						href={`/dashboard/projects/id/${project.id}`}
						aria-label={`Edit ${project.title}`}
					></a>
				</li>
			{/each}
		</ul>
	{/if}
</section>
