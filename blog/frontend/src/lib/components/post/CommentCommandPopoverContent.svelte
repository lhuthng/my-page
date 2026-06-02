<script>
	import CommentGifPopover from './CommentGifPopover.svelte';
	import CommentKaomojiPopover from './CommentKaomojiPopover.svelte';
	import { COMMENT_COMMANDS } from './comment-syntax';

	let { commandState, pickCommandItem, applyKaomojiSuggestion } = $props();

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
			<span>Searching {commandState.type === COMMENT_COMMANDS.GIF ? 'GIFs' : 'Kaomojis'}</span>
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
			{#if commandState.type === COMMENT_COMMANDS.GIF}
				Type to search GIFs... (e.g. /gif cats)
			{:else if commandState.type === COMMENT_COMMANDS.KAOMOJI}
				Type a mood to search Kaomojis... (e.g. /kao happy)
			{/if}
		</p>
	{/if}

	<div bind:this={commandContainer} class="space-y-2">
		{#if commandState.type === COMMENT_COMMANDS.KAOMOJI}
			<CommentKaomojiPopover {commandState} {pickCommandItem} {applyKaomojiSuggestion} />
		{:else if commandState.type === COMMENT_COMMANDS.GIF}
			<CommentGifPopover {commandState} {pickCommandItem} />
		{/if}
	</div>
</div>
