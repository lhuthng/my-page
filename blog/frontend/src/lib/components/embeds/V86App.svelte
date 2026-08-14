<script>
	import { onMount } from 'svelte';
	import { fade } from 'svelte/transition';
	import V86Player from '../project/V86Player.svelte';

	let { name, width = '100%', height = '520px' } = $props();

	let state = $state('loading');
	let errorMessage = $state('');
	let runtime = $state(null);
	let title = $state(name);
	let loaded = $state(false);
	let beforeDemoPortal = $state();
	let afterDemoPortal = $state();

	onMount(async () => {
		try {
			const res = await fetch(`/api/projects/s/${encodeURIComponent(name)}`);
			if (!res.ok) throw new Error('Project demo not found.');
			const data = await res.json();
			if (data.demo_type !== 'v86' || !data.v86_runtime) {
				throw new Error('This project does not have a v86 game.');
			}
			runtime = data.v86_runtime;
			title = data.title || name;
			state = 'ready';
		} catch (error) {
			state = 'error';
			errorMessage = error?.message ?? 'Cannot load v86 project.';
		}
	});
</script>

<div class="relative mx-auto bg-background rounded-lg" style:width style:max-width={width}>
	<div class="mx-auto max-w-full" bind:this={beforeDemoPortal} style:width></div>

	<div
		class="relative max-w-full mx-auto overflow-hidden rounded-xl bg-background"
		style:width
		style:height
	>
		{#if state === 'ready'}
			{#if !loaded}
				<div class="absolute inset-0 grid place-items-center">
					<div class="duo-btn" data-duo-color="green">
						<button onclick={() => (loaded = true)}>Start game</button>
					</div>
				</div>
			{:else}
				<div in:fade={{ duration: 250 }} class="h-full w-full">
					<V86Player {title} {runtime} {beforeDemoPortal} {afterDemoPortal} showKeys={false} />
				</div>
			{/if}
		{:else if state === 'error'}
			<div class="absolute inset-0 grid place-items-center p-4 text-center text-dark/70">
				<p>{errorMessage}</p>
			</div>
		{:else}
			<div class="absolute inset-0 grid place-items-center text-dark/70" role="status">
				Loading game…
			</div>
		{/if}
	</div>

	<div bind:this={afterDemoPortal} class="mx-auto max-w-full" style:width></div>
</div>
