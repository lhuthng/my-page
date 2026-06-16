<script>
	let {
		show,
		kaomojiMood,
		onKaomojiMoodInput,
		kaomojiResults,
		kaomojiSuggestions,
		kaomojiLoading,
		kaomojiTotal,
		kaomojiError,
		fetchKaomojis,
		selectKaomoji,
		applyKaomojiMoodSuggestion
	} = $props();
</script>

{#if show}
	<div class="border-t-2 border-primary/20 bg-white/40 p-3 flex flex-col gap-3">
		<div class="flex gap-2 h-9">
			<input
				type="text"
				value={kaomojiMood}
				placeholder="Search mood..."
				class="flex-1 bg-white border border-primary/30 rounded-xl px-2 py-1 text-base text-dark placeholder-dark/50 outline-none focus:border-primary focus:border-2 transition-colors"
				oninput={(e) => onKaomojiMoodInput?.(e.currentTarget.value)}
				onkeydown={(e) => {
					if (e.key === 'Enter') {
						e.preventDefault();
						fetchKaomojis(true);
					}
				}}
			/>
			<div class="ml-auto w-fit duo-btn" data-duo-color="blue">
				<button type="button" onclick={() => fetchKaomojis(true)}>Search</button>
			</div>
		</div>

		{#if kaomojiError}
			<p class="text-xs text-accent-red font-medium">{kaomojiError}</p>
		{/if}

		{#if kaomojiSuggestions.length > 0}
			<div class="flex flex-wrap gap-2">
				{#each kaomojiSuggestions as suggestion (suggestion)}
					<button
						type="button"
						class="rounded-full border border-primary/30 px-2 py-1 text-xs font-semibold hover:bg-primary-20"
						onclick={() => applyKaomojiMoodSuggestion(suggestion)}
					>
						{suggestion}
					</button>
				{/each}
			</div>
		{/if}

		<div class="flex flex-wrap gap-2 max-h-120 overflow-y-auto pr-1 custom-scrollbar">
			{#each kaomojiResults as kaomoji, index (`${kaomoji}-${index}`)}
				<button
					type="button"
					class="rounded-lg border-2 border-primary/10 bg-primary/5 px-2 py-2 text-center text-base text-dark hover:border-primary hover:bg-primary/10 transition-all"
					onclick={() => selectKaomoji(kaomoji)}
				>
					<span class="font-mono text-base">{kaomoji}</span>
				</button>
			{/each}

			{#if kaomojiLoading}
				{#each Array(6) as _}
					<div class="animate-pulse bg-primary/10 rounded-lg h-12"></div>
				{/each}
			{/if}
		</div>

		{#if kaomojiResults.length > 0 && !kaomojiLoading && kaomojiResults.length < kaomojiTotal}
			<button
				type="button"
				class="text-md font-semibold text-primary hover:text-dark mx-auto py-1"
				onclick={() => fetchKaomojis(false)}
			>
				Load More
			</button>
		{/if}
	</div>
{/if}
