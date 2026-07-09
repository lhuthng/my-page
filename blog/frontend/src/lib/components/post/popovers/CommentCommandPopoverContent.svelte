<script>
	let { commandState, pickCommandItem, applyKaomojiSuggestion } = $props();
	let popoverProps = $derived({
		commandState,
		pickCommandItem,
		applyKaomojiSuggestion
	});

	let commandContainer = $state(null);

	$effect(() => {
		if (commandState.open && commandContainer) {
			const buttons = commandContainer.querySelectorAll('button');
			const activeBtn = buttons[commandState.selected];
			if (activeBtn) {
				activeBtn.scrollIntoView({ block: 'nearest' });
			}
		}
	});
</script>

<div class="max-h-72 overflow-y-scroll px-2 custom-scrollbar">
	{#if commandState.loading}
		<div class="flex items-center gap-2 px-2 py-2 text-sm font-medium text-dark/70">
			<span>Searching {commandState.meta?.loadingLabel ?? 'suggestions'}</span>
			<div class="flex items-center gap-1">
				<span
					class="h-1.5 w-1.5 animate-bounce rounded-full bg-primary"
					style="animation-delay: 0ms; animation-duration: 0.6s;"
				></span>
				<span
					class="h-1.5 w-1.5 animate-bounce rounded-full bg-primary"
					style="animation-delay: 150ms; animation-duration: 0.6s;"
				></span>
				<span
					class="h-1.5 w-1.5 animate-bounce rounded-full bg-primary"
					style="animation-delay: 300ms; animation-duration: 0.6s;"
				></span>
			</div>
		</div>
	{/if}

	{#if commandState.error}
		<p class="px-2 py-2 text-sm font-medium text-accent-red">{commandState.error}</p>
	{/if}

	{#if !commandState.loading && !commandState.error && commandState.items.length === 0 && commandState.suggestions.length === 0}
		<p class="px-2 py-2 text-sm italic font-medium text-dark/60">
			{commandState.meta?.emptyText ?? 'Type to search...'}
		</p>
	{/if}

	<div bind:this={commandContainer} class="space-y-2">
		{#if commandState.meta?.PopoverComponent}
			{@const PopoverComponent = commandState.meta.PopoverComponent}
			<PopoverComponent {...popoverProps} />
		{/if}
	</div>
</div>
