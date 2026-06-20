<script>
	import { getGuestIdentity, GUEST_IDENTITIES } from '$lib/features/comments/guest-identities.js';

	let { identity = $bindable(null), user = null } = $props();

	let open = $state(false);
	let dropdownEl = $state(null);

	const selected = $derived(identity ? getGuestIdentity(identity) : null);

	const displayName = $derived(selected ? selected.name : (user?.displayName ?? null));

	$effect(() => {
		if (!open) return;
		const handler = (e) => {
			if (dropdownEl && !dropdownEl.contains(e.target)) {
				open = false;
			}
		};
		const timer = setTimeout(() => window.addEventListener('click', handler), 0);
		return () => {
			clearTimeout(timer);
			window.removeEventListener('click', handler);
		};
	});
</script>

<div bind:this={dropdownEl} class="relative flex items-center text-sm">
	<span>as</span>
	<button
		type="button"
		class="flex items-center gap-2 px-2 py-1.5 transition-colors hover:[&>svg]:fill-primary text-primary hover:text-dark"
		onclick={() => (open = !open)}
	>
		<span>
			{displayName}
		</span>
		<svg
			class="expand-btn h-1.5 w-3 transition-colors block mx-auto fill-primary/40"
			xmlns="http://www.w3.org/2000/svg"
			viewBox="0 0 32 16"
		>
			<polygon
				class="pointer-events-auto cursor-pointer focus:outline-none"
				points="0,0 32,0 16,16"
				role="button"
				tabindex="0"
			/>
		</svg>
	</button>

	{#if open}
		<div
			class="absolute top-full left-0 translate-2 z-50 bg-white border-2 border-primary rounded-xl shadow-lg/20 p-2 min-w-48 text-base"
		>
			{#if user}
				<button
					type="button"
					class={`flex items-center gap-2 px-2 py-1.5 rounded-lg w-full transition-colors border-2 ${!identity ? 'hover:text-white text-accent-green hover:bg-accent-green-light-1 bg-accent-green-light-3/80 border-accent-green' : 'hover:text-white text-primary/80 border-primary/80 hover:bg-primary/80 bg-primary/20'}`}
					onclick={() => {
						identity = null;
						open = false;
					}}
				>
					<span>
						{user.displayName}
						<span class="italic whitespace-nowrap">(origin)</span>
					</span>
				</button>
				<hr class="my-1 border-dark/10" />
			{/if}
			<span>Aliases:</span>
			<div class="grid xxs:grid-cols-2 gap-1">
				{#each GUEST_IDENTITIES as g}
					{@const selected = g.code === identity}
					<button
						type="button"
						class={`flex items-center transition-colors gap-2 px-2 py-1.5 rounded-lg border-2 ${selected ? 'border-accent-green text-accent-green bg-accent-green-light-2/50 hover:bg-accent-green hover:text-white' : 'hover:text-white text-primary/80 border-primary/80 hover:bg-primary/80 bg-primary/20'}`}
						onclick={() => {
							identity = g.code;
							open = false;
						}}
					>
						<span>{g.name}</span>
					</button>
				{/each}
			</div>
		</div>
	{/if}
</div>
