<script>
	import { COMMENT_COMMANDS } from './comment-syntax';

	let {
		commandState,
		mentionState,
		popoverTop = null,
		pickCommandItem,
		applyKaomojiSuggestion,
		pickMention
	} = $props();

	const popoverStyle = $derived(popoverTop == null ? undefined : `top: ${popoverTop}px;`);

	let commandContainer = $state(null);
	let mentionContainer = $state(null);

	$effect(() => {
		if (commandState.open && commandContainer) {
			const buttons = commandContainer.querySelectorAll('button');
			const activeBtn = buttons[commandState.selected];
			if (activeBtn) {
				activeBtn.scrollIntoView({ block: 'nearest' });
			}
		}
	});

	$effect(() => {
		if (mentionState.open && mentionContainer) {
			const buttons = mentionContainer.querySelectorAll('button');
			const activeBtn = buttons[mentionState.selected];
			if (activeBtn) {
				activeBtn.scrollIntoView({ block: 'nearest' });
			}
		}
	});
</script>

{#if commandState.open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="comment-autocomplete-popover absolute left-2 right-2 z-30 rounded-xl border-2 border-primary bg-white py-2 shadow-lg"
		style={popoverStyle}
		onmousedown={(event) => {
			event.preventDefault();
		}}
	>
		<div
			class="overflow-y-scroll h-full max-h-72 custom-scrollbar px-2"
			bind:this={commandContainer}
		>
			{#if commandState.loading}
				<p class="px-2 py-2 text-sm text-dark/70">Searching...</p>
			{/if}

			{#if commandState.error}
				<p class="px-2 py-2 text-sm text-accent-red font-medium">{commandState.error}</p>
			{/if}

			{#if commandState.type === COMMENT_COMMANDS.KAOMOJI && commandState.items.length > 0}
				<div class="flex flex-wrap gap-2">
					{#each commandState.items as item, index (item.id)}
						<button
							type="button"
							class="w-full rounded-lg px-2 py-1 text-left font-mono hover:bg-primary-20"
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

			{#if commandState.type === COMMENT_COMMANDS.GIF && commandState.items.length > 0}
				<div class="grid grid-cols-2 gap-2">
					{#each commandState.items as gif, index (gif.id)}
						<button
							type="button"
							class="flex items-center gap-2 rounded-lg border border-primary/15 p-1 text-left hover:bg-primary-20"
							class:bg-primary-20={index === commandState.selected}
							onmousedown={(event) => {
								event.preventDefault();
								pickCommandItem(gif);
							}}
						>
							<img
								src={gif.images.fixed_height.url}
								alt={gif.title}
								class="h-12 w-12 rounded object-cover"
								loading="lazy"
							/>
							<p class="line-clamp-2 text-sm font-medium leading-tight">{gif.title || 'GIF'}</p>
						</button>
					{/each}
				</div>
			{/if}

			{#if commandState.type === COMMENT_COMMANDS.KAOMOJI && commandState.suggestions.length > 0}
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
					<p class="text-xs text-dark/60">Tip: press Tab to use the first suggestion.</p>
				</div>
			{/if}
		</div>
	</div>
{:else if mentionState.open}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<ul
		class="comment-autocomplete-popover absolute left-2 right-2 z-30 max-h-64 overflow-y-auto rounded-xl border-2 border-primary bg-white p-1 shadow-lg"
		style={popoverStyle}
		onmousedown={(event) => {
			event.preventDefault();
		}}
		bind:this={mentionContainer}
	>
		{#each mentionState.items as profile, index (profile.username)}
			<li>
				<button
					type="button"
					class="flex w-full items-center gap-3 rounded-lg px-2 py-1 text-left hover:bg-primary-20"
					class:bg-primary-20={index === mentionState.selected}
					onmousedown={(event) => {
						event.preventDefault();
						pickMention(profile);
					}}
				>
					<img
						class="h-8 w-8 rounded-full object-cover outline-2 outline-primary"
						src={profile.avatar_url ?? '/anonymous.gif'}
						alt={`${profile.display_name} avatar`}
					/>
					<div class="min-w-0 flex-1">
						<p class="truncate font-semibold">{profile.display_name}</p>
						<p class="truncate text-xs opacity-80">@{profile.username}</p>
					</div>
				</button>
			</li>
		{/each}
	</ul>
{/if}
