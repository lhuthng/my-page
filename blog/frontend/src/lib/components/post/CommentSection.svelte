<script>
	import { authState } from '$lib/auth/user.svelte.js';
	import CommentEditor from './editor/CommentEditor.svelte';
	import CommentFeed from './feed/CommentFeed.svelte';
	import {
		createCommentSectionRuntime,
		createEmptyState
	} from '$lib/features/comments/comment-section-runtime';

	let { postId, postAuthorUsername = null } = $props();
	let last = -1;

	let userAvatarUrl = $derived(authState.user?.avatarUrl ?? '/anonymous.gif');

	let state = $state(createEmptyState());
	const comments = state.comments;
	const replyThreads = state.replyThreads;
	const mentionState = state.mentionState;
	const commandState = state.commandState;
	const runtime = createCommentSectionRuntime(state, () => userAvatarUrl);

	$effect(() => {
		if (last !== postId) {
			last = postId;
			return runtime.handlePostChange(postId);
		}
	});

	const handleComposerInput = runtime.handleComposerInput;
	const handleTextareaBlur = runtime.handleTextareaBlur;
	const handleTextareaKeydown = runtime.handleTextareaKeydown;
	const fetchComments = runtime.fetchComments;
	const handleReply = runtime.handleReply;
	const toggleReplies = runtime.toggleReplies;
	const loadMoreReplies = runtime.loadMoreReplies;
	const submitComment = runtime.submitComment;
	const toggleMarkdownHelp = runtime.toggleMarkdownHelp;
	const toggleKaomojiDrawer = runtime.toggleKaomojiDrawer;
	const toggleGifDrawer = runtime.toggleGifDrawer;
	const closeMarkdownHelp = runtime.closeMarkdownHelp;
	const insertAtCursor = runtime.insertAtCursor;
	const wrapSelection = runtime.wrapSelection;
	const pickCommandItem = runtime.pickCommandItem;
	const applyKaomojiSuggestion = runtime.applyKaomojiSuggestion;
	const pickMention = runtime.pickMention;
	const schedulePopoverAnchor = runtime.schedulePopoverAnchor;
	const fetchGifs = runtime.fetchGifs;
	const fetchKaomojis = runtime.fetchKaomojis;
	const selectGif = runtime.selectGif;
	const selectKaomoji = runtime.selectKaomoji;
	const applyKaomojiMoodSuggestion = runtime.applyKaomojiMoodSuggestion;
</script>

<section class="w-full xl:max-w-[calc(100%-15rem)] xl:w-[calc(100%-15rem)] bg-white p-4 rounded-xl">
	<h4 class="text-lg lg:text-2xl">Join the discussion!</h4>
	<div class="flex flex-col gap-4" bind:this={state.start}>
		<hr class="border-t-3 border-dark mb-6" />
		<CommentEditor
			bind:textarea={state.textarea}
			bind:composerSurface={state.composerSurface}
			bind:value={comments.current}
			{userAvatarUrl}
			previewHtml={comments.current.trim().length > 0 ? runtime.md.render(comments.current) : ''}
			replyTo={state.replyTo}
			sending={comments.sending}
			{commandState}
			{mentionState}
			popoverTop={state.popoverTop}
			showMarkdownHelp={state.showMarkdownHelp}
			showKaomojiSearch={state.showKaomojiSearch}
			showGifSearch={state.showGifSearch}
			gifQuery={state.gifQuery}
			onGifQueryInput={(value) => {
				state.gifQuery = value;
			}}
			gifResults={state.gifResults}
			gifLoading={state.gifLoading}
			gifError={state.gifError}
			{fetchGifs}
			{selectGif}
			kaomojiMood={state.kaomojiMood}
			onKaomojiMoodInput={(value) => {
				state.kaomojiMood = value;
			}}
			kaomojiResults={state.kaomojiResults}
			kaomojiSuggestions={state.kaomojiSuggestions}
			kaomojiLoading={state.kaomojiLoading}
			kaomojiTotal={state.kaomojiTotal}
			kaomojiError={state.kaomojiError}
			{fetchKaomojis}
			{selectKaomoji}
			{applyKaomojiMoodSuggestion}
			onComposerInput={handleComposerInput}
			onPopoverScroll={schedulePopoverAnchor}
			onTextareaKeydown={handleTextareaKeydown}
			onTextareaBlur={handleTextareaBlur}
			{pickCommandItem}
			{applyKaomojiSuggestion}
			{pickMention}
			onToggleHelp={toggleMarkdownHelp}
			onHeader={() => insertAtCursor('# ', 2)}
			onBold={() => wrapSelection('**')}
			onItalic={() => wrapSelection('_')}
			onCode={() => wrapSelection('`')}
			onToggleKaomoji={toggleKaomojiDrawer}
			onToggleGif={toggleGifDrawer}
			onCloseMarkdownHelp={closeMarkdownHelp}
			onCancelReply={() => {
				state.replyTo = null;
			}}
			onSubmit={submitComment}
		/>
	</div>
	<CommentFeed
		comments={comments.roots}
		fetching={comments.fetching}
		endReached={comments.endReached}
		{postAuthorUsername}
		{replyThreads}
		onReply={handleReply}
		onToggleReplies={toggleReplies}
		onLoadMoreReplies={loadMoreReplies}
		onLoadMore={fetchComments}
	/>
</section>
