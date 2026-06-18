<script>
	import { onMount } from 'svelte';
	import { fade, fly } from 'svelte/transition';

	let { name, type, width, height } = $props();

	let loaded = $state(false);
	let src = $state('');
	let errorMessage = $state('');
	let resolving = $state(true);

	onMount(async () => {
		if (type !== 'project' && type !== 'html') return;

		try {
			const res = await fetch(`/api/projects/s/${encodeURIComponent(name)}`);
			if (!res.ok) {
				errorMessage = 'Project demo not found.';
				return;
			}

			const data = await res.json();
			if (data.demo_url?.includes('://')) {
				src = data.demo_url;
			} else {
				src = data.demo_url?.startsWith('/api/') ? data.demo_url : `/api/${data.demo_url}`;
			}
			if (!data.demo_url) {
				errorMessage = 'Project demo is missing.';
			}
		} catch (_) {
			errorMessage = 'Cannot load project demo right now.';
		} finally {
			resolving = false;
		}
	});
</script>

<div
	class="relative mx-auto bg-background rounded-lg"
	style:width
	style:max-width={width}
	style:height
>
	{#if loaded && src}
		<iframe
			in:fade={{ duration: 800 }}
			class="rounded-md"
			style:width
			style:height
			{src}
			title={name}
			frameborder="0"
		></iframe>
	{:else if errorMessage}
		<div class="absolute inset-0 grid place-items-center p-4 text-center text-dark/70">
			<p>{errorMessage}</p>
		</div>
	{:else}
		<div out:fly={{ y: 20, duration: 800 }} class="absolute top-1/2 left-1/2 -translate-1/2">
			<div class="w-fit duo-btn" data-duo-color="green">
				<button onclick={() => (loaded = true)} disabled={!src}>
					{resolving ? 'Loading Demo...' : 'Start App'}
				</button>
			</div>
		</div>
	{/if}
</div>
