<script>
	let { mentionState, pickMention } = $props();

	let mentionContainer = $state(null);

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

<ul bind:this={mentionContainer} class="custom-scrollbar">
	{#if mentionState.loading}
		<li class="searching">Searching...</li>
	{/if}

	{#each mentionState.items as profile, index (profile.username)}
		<li>
			<button
				type="button"
				class:bg-primary-20={index === mentionState.selected}
				onmousedown={(event) => {
					event.preventDefault();
					pickMention(profile);
				}}
			>
				<img
					class=""
					src={profile.avatar_url ?? '/anonymous.gif'}
					alt={`${profile.display_name} avatar`}
				/>
				<div>
					<p class="font-semibold">{profile.display_name}</p>
					<p class="text-xs opacity-80">@{profile.username}</p>
				</div>
			</button>
		</li>
	{/each}
</ul>

<style lang="postcss">
	@reference "../../../../app.css";
	ul {
		@apply max-h-64 overflow-y-auto rounded-xl bg-white px-2 shadow-lg;
		& > li.searching {
			@apply px-2 py-2 text-sm text-dark/70;
		}
		& > li > button {
			@apply flex w-full items-center gap-3 rounded-lg px-2 py-1 text-left hover:bg-primary-20;
			& > img {
				@apply h-8 w-8 rounded-full object-cover outline-2 outline-primary;
			}
			& > div {
				@apply min-w-0 flex-1 [&>p]:truncate;
			}
		}
	}
</style>
