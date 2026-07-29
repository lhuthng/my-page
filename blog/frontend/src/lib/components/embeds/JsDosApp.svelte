<script>
	import { onMount } from 'svelte';
	import JsDosPlayer from '../project/JsDosPlayer.svelte';

	let { name, width = '100%', height = '520px' } = $props();
	let bundleUrl = $state('');
	let state = $state('loading');
	let errorMessage = $state('');

	onMount(async () => {
		try {
			const res = await fetch(`/api/projects/s/${encodeURIComponent(name)}`);
			if (!res.ok) throw new Error('Project demo not found.');
			const data = await res.json();
			if (data.demo_type !== 'jsdos' || !data.demo_url) {
				throw new Error('This project does not have a js-dos game.');
			}
			bundleUrl = data.demo_url.startsWith('/api/') ? data.demo_url : `/api/${data.demo_url}`;
			state = 'ready';
		} catch (error) {
			state = 'error';
			errorMessage = error?.message ?? 'Cannot load js-dos project.';
		}
	});
</script>

<div class="relative mx-auto overflow-hidden bg-black" style:width style:height>
	{#if state === 'ready'}
		<JsDosPlayer title={name} {bundleUrl} />
	{:else if state === 'error'}
		<div class="grid h-full place-items-center p-4 text-center text-dark/70">
			<p>{errorMessage}</p>
		</div>
	{:else}
		<div class="grid h-full place-items-center text-white" role="status">Loading game…</div>
	{/if}
</div>
