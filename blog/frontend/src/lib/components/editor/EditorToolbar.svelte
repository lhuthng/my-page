<script>
	import { fly } from 'svelte/transition';

	let {
		toggled = $bindable(false),
		view = 'private',
		status = '',
		isCritical = false,
		isPublishing = false,
		mode = 'create',
		isOwner = true,
		ontoggle = () => {},
		ontogglevision = () => {},
		onsubmit = () => {},
		onchange = () => {},
		onpublish = () => {}
	} = $props();
</script>

<div class="flex justify-between p-2 w-full">
	<div class="flex gap-2">
		<div class="w-25 duo-btn" data-duo-color={toggled ? 'red' : 'green'}>
			<button onclick={ontoggle}>{toggled ? 'Collapse' : 'Expand'}</button>
		</div>
		{#if mode === 'edit' && isOwner}
			<div in:fly={{ duration: 200 }} class="duo-btn" data-duo-color="blue">
				<button onclick={ontogglevision}>
					Ver. {view === 'public' ? 'Published' : 'Draft'}
				</button>
			</div>
		{/if}
	</div>
	<div class="flex gap-2">
		{#if toggled}
			<div class="my-auto" class:text-accent-green={!isCritical} class:text-accent-red={isCritical}>
				<span>{status}</span>
			</div>
			{#if mode === 'create'}
				<div in:fly={{ duration: 200 }} class="duo-btn" data-duo-color="green">
					<button onclick={onsubmit}>Submit</button>
				</div>
			{:else if !isOwner}
				<div class="my-auto text-dark/50 text-sm italic">
					<span>View only</span>
				</div>
			{:else}
				<div in:fly={{ duration: 200 }} class="duo-btn" data-duo-color="green">
					<button onclick={onchange}>Change</button>
				</div>
				<div in:fly={{ duration: 200 }} class="duo-btn" data-duo-color="green">
					<button onclick={onpublish} disabled={isPublishing}>
						{isPublishing ? 'Publishing\u2026' : 'Publish'}
					</button>
				</div>
			{/if}
		{/if}
	</div>
</div>
