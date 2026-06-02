<script>
	import CommentAutocompletePopoverContainer from './CommentAutocompletePopoverContainer.svelte';
	import CommentCommandPopoverContent from './CommentCommandPopoverContent.svelte';
	import CommentMentionPopoverContent from './CommentMentionPopoverContent.svelte';

	let {
		commandState,
		mentionState,
		popoverTop = null,
		pickCommandItem,
		applyKaomojiSuggestion,
		pickMention
	} = $props();
</script>

{#if commandState.open}
	<CommentAutocompletePopoverContainer open={commandState.open} top={popoverTop}>
		<CommentCommandPopoverContent {commandState} {pickCommandItem} {applyKaomojiSuggestion} />
	</CommentAutocompletePopoverContainer>
{:else if mentionState.open}
	<CommentAutocompletePopoverContainer open={mentionState.open} top={popoverTop} className="py-0">
		<CommentMentionPopoverContent {mentionState} {pickMention} />
	</CommentAutocompletePopoverContainer>
{/if}
