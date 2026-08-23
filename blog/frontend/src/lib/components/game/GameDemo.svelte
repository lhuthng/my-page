<script>
	import ProjectDemo from '../project/ProjectDemo.svelte';

	let {
		title,
		launcherType = 'html5',
		demoUrl,
		v86Runtime,
		width = '100%',
		height = '520px',
		instruction = '',
		cheatcode = '',
		story = '',
		initialVariant,
		children
	} = $props();

	const extras = $derived(
		[
			{ title: 'How to play', body: instruction },
			{ title: 'Cheats & secrets', body: cheatcode },
			{ title: 'Story', body: story }
		].filter((section) => section.body.trim() !== '')
	);
</script>

<ProjectDemo
	{title}
	demoType={launcherType}
	{demoUrl}
	{v86Runtime}
	{initialVariant}
	{width}
	{height}
	backHref="/games"
	backLabel="All games"
>
	{#each extras as section (section.title)}
		<div class="mt-4 rounded-xl border border-dark/10 bg-background/30 p-3">
			<h3 class="mb-1 text-sm font-semibold uppercase tracking-wide text-dark/50">
				{section.title}
			</h3>
			<p class="whitespace-pre-wrap text-sm leading-relaxed text-dark/80">{section.body}</p>
		</div>
	{/each}

	{@render children?.()}
</ProjectDemo>
