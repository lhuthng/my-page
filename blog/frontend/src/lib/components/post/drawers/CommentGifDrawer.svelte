<script>
	let { show, gifQuery, onGifQueryInput, gifResults, gifLoading, gifError, fetchGifs, selectGif } =
		$props();
</script>

{#if show}
	<div class="border-t-2 border-primary/20 bg-white/40 p-3 flex flex-col gap-3">
		<div class="flex gap-2 h-9">
			<input
				type="text"
				value={gifQuery}
				placeholder="Search Giphy..."
				class="flex-1 bg-white border border-primary/30 rounded-xl px-2 py-1 text-base text-dark placeholder-dark/50 outline-none focus:border-primary focus:border-2 transition-colors"
				oninput={(e) => onGifQueryInput?.(e.currentTarget.value)}
				onkeydown={(e) => {
					if (e.key === 'Enter') {
						e.preventDefault();
						fetchGifs(true);
					}
				}}
			/>
			<div class="ml-auto w-fit duo-btn" data-duo-color="blue">
				<button type="button" onclick={() => fetchGifs(true)}>Search</button>
			</div>
		</div>

		{#if gifError}
			<p class="text-xs text-accent-red font-medium">{gifError}</p>
		{/if}

		<div
			class="grid grid-cols-3 xs:grid-cols-4 sm:grid-cols-6 gap-2 max-h-120 overflow-y-auto pr-1 custom-scrollbar"
		>
			{#each gifResults as gif (gif.id)}
				<button
					type="button"
					class="group relative rounded-lg overflow-hidden aspect-square border border-primary/10 hover:border-primary transition-all bg-dark/5"
					onclick={() => selectGif(gif)}
				>
					<img
						src={gif.images.fixed_height.url}
						alt={gif.title}
						class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-200"
						loading="lazy"
					/>
					<div
						class="absolute inset-0 bg-primary/70 opacity-0 group-hover:opacity-100 flex items-center justify-center font-bold text-xl text-white transition-opacity duration-150"
					>
						Select
					</div>
				</button>
			{/each}

			{#if gifLoading}
				{#each Array(6) as _}
					<div class="animate-pulse bg-primary/10 rounded-lg aspect-square"></div>
				{/each}
			{/if}
		</div>

		{#if gifResults.length > 0 && !gifLoading}
			<button
				type="button"
				class="text-md font-semibold text-primary hover:text-dark mx-auto py-1"
				onclick={() => fetchGifs(false)}
			>
				Load More
			</button>
		{/if}
	</div>
{/if}
