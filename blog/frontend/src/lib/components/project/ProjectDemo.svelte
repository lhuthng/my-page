<script>
	import { fade, fly } from 'svelte/transition';
	import BackButton from '../ui/BackButton.svelte';
	import JsDosPlayer from './JsDosPlayer.svelte';

	let { title, demoType = 'html5', demoUrl, width = '100%', height = '520px' } = $props();
	let loaded = $state(false);
</script>

<section class="bg-white rounded-xl p-4 drop-shadow-xl">
	<div class="space-y-2">
		<BackButton href="/projects" text="All projects" />
		<div class="flex items-center gap-3 mb-3">
			<h2 class="text-xl lg:text-2xl">Demo</h2>
			<hr class="grow border" />
		</div>
	</div>
	<div
		class="relative mx-auto bg-background rounded-lg overflow-hidden"
		style:width
		style:max-width="100%"
		style:height
	>
		{#if demoType === 'jsdos'}
			<JsDosPlayer {title} bundleUrl={demoUrl} />
		{:else if demoType === 'video'}
			<video class="block w-full h-full object-contain bg-black" src={demoUrl} controls {title}>
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
			{#if loaded}
				<iframe
					in:fade={{ duration: 500 }}
					class="block w-full h-full bg-white"
					src={demoUrl}
					title={`${title} demo`}
					scrolling="no"
					frameborder="0"
				></iframe>
			{:else}
				<div out:fly={{ y: 20, duration: 300 }} class="absolute inset-0 grid place-items-center">
					<div class="duo-btn" data-duo-color="green">
						<button onclick={() => (loaded = true)}>
							{demoType === 'webgl' ? 'Launch WebGL' : 'Start Demo'}
						</button>
					</div>
				</div>
			{/if}
		{/if}
	</div>
</section>
