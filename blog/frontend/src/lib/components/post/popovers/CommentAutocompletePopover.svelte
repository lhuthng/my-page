<script>
	import CommentAutocompletePopoverContainer from './CommentAutocompletePopoverContainer.svelte';
	import CommentCommandPopoverContent from './CommentCommandPopoverContent.svelte';
	import CommentMentionPopoverContent from './CommentMentionPopoverContent.svelte';

	let {
		commandState,
		mentionState,
		suggestionPanel = null,
		popoverTop = null,
		pickCommandItem,
		applyKaomojiSuggestion,
		pickMention
	} = $props();
</script>

{#if suggestionPanel?.mode === 'command'}
	<CommentAutocompletePopoverContainer
		open={suggestionPanel.open}
		top={suggestionPanel.top ?? popoverTop}
	>
		<CommentCommandPopoverContent {commandState} {pickCommandItem} {applyKaomojiSuggestion} />
	</CommentAutocompletePopoverContainer>
{:else if suggestionPanel?.mode === 'mention'}
	<CommentAutocompletePopoverContainer
		open={suggestionPanel.open}
		top={suggestionPanel.top ?? popoverTop}
		className="py-0"
	>
		<CommentMentionPopoverContent {mentionState} {pickMention} />
	</CommentAutocompletePopoverContainer>
{/if}
