<script>
	import { fade } from 'svelte/transition';
	import BackButton from '../ui/BackButton.svelte';
	import JsDosPlayer from './JsDosPlayer.svelte';
	import V86Player from './V86Player.svelte';

	let {
		title,
		demoType = 'html5',
		demoUrl,
		v86Runtime,
		width = '100%',
		height = '520px',
		...rest
	} = $props();

	let loaded = $state(false);
	let beforeDemoPortal = $state();
	let afterDemoPortal = $state();
	let startsOnDemand = $derived(['html5', 'embed', 'webgl', 'jsdos', 'v86'].includes(demoType));
	let isGame = $derived(demoType === 'jsdos' || demoType === 'v86');
	let startLabel = $derived(
		isGame ? 'Start game' : demoType === 'webgl' ? 'Launch WebGL' : 'Start Demo'
	);
</script>

<section class="bg-white rounded-xl p-4 drop-shadow-xl">
	<div class="space-y-2">
		<BackButton href="/projects" text="All projects" />
		<div class="flex items-center gap-3 mb-3">
			<h2 class="text-xl lg:text-2xl">Demo</h2>
			<hr class="grow border" />
		</div>
	</div>
	<div class="mx-auto max-w-full" bind:this={beforeDemoPortal} style:width></div>

	<div
		class="relative max-w-full mx-auto overflow-hidden rounded-xl bg-background"
		style:width
		style:height
	>
		{#if startsOnDemand && !loaded}
			<div class="absolute inset-0 grid place-items-center">
				<div class="duo-btn" data-duo-color="green">
					<button onclick={() => (loaded = true)}>{startLabel}</button>
				</div>
			</div>
		{:else if demoType === 'jsdos'}
			<div in:fade={{ duration: 250 }} class="h-full w-full">
				<JsDosPlayer {title} bundleUrl={demoUrl} {beforeDemoPortal} {afterDemoPortal} />
			</div>
		{:else if demoType === 'v86'}
			<div in:fade={{ duration: 250 }} class="h-full w-full">
				<V86Player {title} runtime={v86Runtime} {...rest} {beforeDemoPortal} {afterDemoPortal} />
			</div>
		{:else if demoType === 'video'}
			<video class="block h-full w-full bg-black object-contain" src={demoUrl} controls {title}>
				<track kind="captions" src="" srclang="en" label="English captions" default />
			</video>
		{:else if demoType === 'download'}
			<div class="absolute inset-0 grid place-items-center p-6 text-center">
				<div class="space-y-4">
					<p class="text-dark/80 text-lg">This project is available for download.</p>
					<div class="duo-btn text-xl" data-duo-color="green">
						<a
							href={demoUrl}
							download
							target="_blank"
							rel="noopener noreferrer"
							class="no-underline! block px-8 py-3"
						>
							Download Project
						</a>
					</div>
				</div>
			</div>
		{:else}
			<iframe
				in:fade={{ duration: 250 }}
				class="block h-full w-full bg-white"
				src={demoUrl}
				title={`${title} demo`}
				scrolling="no"
				frameborder="0"
			></iframe>
		{/if}
	</div>

	<div bind:this={afterDemoPortal} class="mx-auto max-w-full" style:width></div>
</section>
