<script>
	import { fade, fly } from 'svelte/transition';

	let { title, demoUrl, width = '100%', height = '520px' } = $props();
	let loaded = $state(false);
</script>

<section class="bg-white rounded-xl p-4 drop-shadow-xl">
	<div class="flex items-center gap-3 mb-3">
		<h2 class="text-xl lg:text-2xl">Demo</h2>
		<hr class="grow border" />
	</div>
	<div
		class="relative mx-auto bg-background rounded-lg overflow-hidden"
		style:width
		style:max-width="100%"
		style:height
	>
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
				<div class="duo-btn duo-green">
					<button onclick={() => (loaded = true)}>Start Demo</button>
				</div>
			</div>
		{/if}
	</div>
</section>
