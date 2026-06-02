<script>
	import CommentThread from './CommentThread.svelte';

	let {
		comments = [],
		fetching = false,
		endReached = false,
		postAuthorUsername = null,
		replyThreads = {},
		onReply,
		onToggleReplies,
		onLoadMoreReplies,
		onLoadMore
	} = $props();
</script>

{#if endReached && comments.length === 0 && !fetching}
	<div class="text-center py-8 text-dark/60">
		<p class="text-lg">
			No comments yet be the first! <span class="whitespace-nowrap">(⌒▽⌒)☆</span>
		</p>
	</div>
{/if}

<CommentThread
	{comments}
	{postAuthorUsername}
	{replyThreads}
	{onReply}
	{onToggleReplies}
	{onLoadMoreReplies}
/>

<div class="mt-8 mx-auto w-fit duo-btn duo-blue">
	<button disabled={endReached || fetching} onclick={onLoadMore}>
		{endReached ? 'No more to read ٩(｡•́‿•̀｡)۶' : 'Read more'}
	</button>
</div>
