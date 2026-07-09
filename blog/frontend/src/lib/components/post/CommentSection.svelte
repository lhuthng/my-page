<script>
	import { authState } from '$lib/auth/user.svelte.js';
	import CommentEditor from './editor/CommentEditor.svelte';
	import CommentFeed from './feed/CommentFeed.svelte';
	import { getGuestIdentity, GUEST_IDENTITIES } from '$lib/features/comments/guest-identities.js';
	import {
		createCommentSectionRuntime,
		createEmptyState
	} from '$lib/features/comments/comment-section-runtime';
	import { onMount } from 'svelte';
	import { browser } from '$app/env';
	import { fade } from 'svelte/transition';
	import GridExpander from '../shell/GridExpander.svelte';

	let { postId, postAuthorUsername = null } = $props();
	let last = -1;

	const initIndex = Math.floor(GUEST_IDENTITIES.length * Math.random());

	let guestIdentity = $state(authState.user ? null : GUEST_IDENTITIES[initIndex].code);
	let userAvatarUrl = $derived(
		guestIdentity
			? (getGuestIdentity(guestIdentity)?.avatar ?? '/anonymous.gif')
			: (authState.user?.avatarUrl ?? '/anonymous.gif')
	);

	let state = $state(createEmptyState());
	const comments = state.comments;
	const replyThreads = state.replyThreads;
	const runtime = createCommentSectionRuntime(
		state,
		() => userAvatarUrl,
		() => guestIdentity,
		() => {}
	);

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
	const pickCommandItem = runtime.pickCommandItem;
	const applyKaomojiSuggestion = runtime.applyKaomojiSuggestion;
	const pickMention = runtime.pickMention;
	const schedulePopoverAnchor = runtime.schedulePopoverAnchor;
	const fetchGifs = runtime.fetchGifs;
	const fetchKaomojis = runtime.fetchKaomojis;
	const selectGif = runtime.selectGif;
	const selectKaomoji = runtime.selectKaomoji;
	const applyKaomojiMoodSuggestion = runtime.applyKaomojiMoodSuggestion;

	let mounted = $state(false);
	onMount(() => (mounted = true));
</script>

<section class="w-full xl:max-w-[calc(100%-15rem)] xl:w-[calc(100%-15rem)] bg-white p-4 rounded-xl">
	<h4 class="text-lg lg:text-2xl">Join the discussion!</h4>
	<hr class="border-t-3 border-dark mb-6" />
	<GridExpander class="pb-2" expanded={mounted} duration="300ms">
		{#if browser}
			<div class="flex flex-col gap-4" bind:this={state.start}>
				{#if comments.commentError}
					<div class="text-accent-red text-sm mb-2 px-2" transition:fade>
						{comments.commentError}
					</div>
				{/if}
				<CommentEditor
					bind:textarea={state.textarea}
					bind:composerSurface={state.composerSurface}
					bind:popoverSurface={state.popoverSurface}
					bind:value={comments.current}
					bind:guestIdentity
					user={authState.user}
					{userAvatarUrl}
					previewHtml={comments.current.trim().length > 0
						? runtime.md.render(comments.current)
						: ''}
					replyTo={state.composer.replyTo}
					sending={comments.sending}
					commandState={state.commandState}
					mentionState={state.mentionState}
					suggestionPanel={runtime.suggestionPanel}
					popoverTop={state.ui.popoverTop}
					showMarkdownHelp={state.ui.showMarkdownHelp}
					showKaomojiSearch={state.drawers.showKaomojiSearch}
					showGifSearch={state.drawers.showGifSearch}
					gifQuery={state.drawers.gifQuery}
					onGifQueryInput={(value) => {
						state.drawers.gifQuery = value;
					}}
					gifResults={state.drawers.gifResults}
					gifLoading={state.drawers.gifLoading}
					gifError={state.drawers.gifError}
					{fetchGifs}
					{selectGif}
					kaomojiMood={state.drawers.kaomojiMood}
					onKaomojiMoodInput={(value) => {
						state.drawers.kaomojiMood = value;
					}}
					kaomojiResults={state.drawers.kaomojiResults}
					kaomojiSuggestions={state.drawers.kaomojiSuggestions}
					kaomojiLoading={state.drawers.kaomojiLoading}
					kaomojiTotal={state.drawers.kaomojiTotal}
					kaomojiError={state.drawers.kaomojiError}
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
					onHeader={runtime.insertHeader}
					onBold={runtime.wrapBold}
					onItalic={runtime.wrapItalic}
					onCode={runtime.wrapCode}
					onToggleKaomoji={toggleKaomojiDrawer}
					onToggleGif={toggleGifDrawer}
					onCloseMarkdownHelp={closeMarkdownHelp}
					onCancelReply={() => {
						state.composer.replyTo = null;
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
		{/if}
	</GridExpander>
</section>
