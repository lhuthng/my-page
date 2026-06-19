<script>
	let { commandState, pickCommandItem, applyKaomojiSuggestion } = $props();
</script>

{#if commandState.items.length > 0}
	<div class="flex flex-wrap gap-2">
		{#each commandState.items as item, index (item.id)}
			<button
				type="button"
				class="w-fit rounded-lg px-2 py-1 text-left font-mono hover:bg-primary-20"
				class:bg-primary-20={index === commandState.selected}
				onmousedown={(event) => {
					event.preventDefault();
					pickCommandItem(item);
				}}
			>
				{item.value}
			</button>
		{/each}
	</div>
{/if}

{#if commandState.suggestions.length > 0}
	<div class="space-y-2 px-1 py-1">
		<p class="text-sm text-dark/70">Mood not found. Try one of these suggestions:</p>
		<div class="flex flex-wrap gap-2">
			{#each commandState.suggestions as suggestion, index (suggestion)}
				<button
					type="button"
					class="rounded-full border border-primary/30 px-3 py-1 text-sm font-semibold hover:bg-primary-20"
					class:bg-primary-20={index === commandState.selected}
					onmousedown={(event) => {
						event.preventDefault();
						applyKaomojiSuggestion(suggestion);
					}}
				>
					{suggestion}
				</button>
			{/each}
		</div>
		<p class="italic">Tip: press Tab to use the first suggestion.</p>
	</div>
{/if}
