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
/>

<!-- Each note is its own card with the site's standard section header, rather
     than a tinted box nested inside the demo card. -->
{#each extras as section (section.title)}
	<section class="bg-white rounded-xl p-4">
		<div class="space-y-2">
			<div class="flex items-center gap-3 mb-3">
				<h2 class="text-xl lg:text-2xl">{section.title}</h2>
				<hr class="grow border" />
			</div>
		</div>
		<p class="whitespace-pre-wrap text-base leading-relaxed text-dark/80">{section.body}</p>
	</section>
{/each}

{@render children?.()}
