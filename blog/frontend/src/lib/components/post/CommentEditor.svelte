<script>
	import { autoHResize } from '$lib/client/auto-resize';
	import Comment from './Comment.svelte';
	import CommentAutocompletePopover from './CommentAutocompletePopover.svelte';
	import CommentComposerToolbar from './CommentComposerToolbar.svelte';
	import CommentGifDrawer from './CommentGifDrawer.svelte';
	import CommentKaomojiDrawer from './CommentKaomojiDrawer.svelte';
	import CommentMarkdownHelp from './CommentMarkdownHelp.svelte';

	let {
		textarea = $bindable(),
		composerSurface = $bindable(),
		value = $bindable(''),
		userAvatarUrl = '/anonymous.gif',
		previewHtml = '',
		replyTo = null,
		sending = false,
		commandState,
		mentionState,
		popoverTop = null,
		showMarkdownHelp = false,
		showKaomojiSearch = false,
		showGifSearch = false,
		gifQuery = '',
		onGifQueryInput,
		gifResults = [],
		gifLoading = false,
		gifError = null,
		fetchGifs,
		selectGif,
		kaomojiMood = '',
		onKaomojiMoodInput,
		kaomojiResults = [],
		kaomojiSuggestions = [],
		kaomojiLoading = false,
		kaomojiTotal = 0,
		kaomojiError = null,
		fetchKaomojis,
		selectKaomoji,
		applyKaomojiMoodSuggestion,
		onComposerInput,
		onPopoverScroll,
		onTextareaKeydown,
		onTextareaBlur,
		pickCommandItem,
		applyKaomojiSuggestion,
		pickMention,
		onToggleHelp,
		onHeader,
		onBold,
		onItalic,
		onCode,
		onToggleKaomoji,
		onToggleGif,
		onCloseMarkdownHelp,
		onCancelReply,
		onSubmit
	} = $props();
</script>

<div class="flex gap-8">
	<div
		class="not-md:hidden min-w-12 max-w-12 lg:min-w-20 lg:max-w-20 h-12 lg:h-20 outline-primary outline-3 rounded-full overflow-hidden"
	>
		<img class="full object-cover" src={userAvatarUrl} alt="comment-posting-avatar" />
	</div>

	<div class="grow min-w-0 flex flex-col gap-4 relative">
		<svg
			class="not-md:hidden absolute fill-primary top-6 lg:top-10 -left-4 -translate-y-1/2 w-4 h-4"
			viewBox="0 0 24 24"
		>
			<polygon points="0,12 24,0 24,24" />
		</svg>

		<div class="w-full max-w-full">
			<div
				class="relative text-base w-full bg-primary-20 border-primary border-2 border-b-0 rounded-t-xl"
				bind:this={composerSurface}
			>
				<textarea
					name="comment-input"
					class="comment-input block w-full min-h-16 lg:min-h-20 max-w-full overflow-hidden outline-none resize-none p-2 bg-transparent"
					wrap="soft"
					bind:this={textarea}
					bind:value
					oninput={onComposerInput}
					onclick={onComposerInput}
					onkeyup={onComposerInput}
					onscroll={onPopoverScroll}
					onkeydown={onTextareaKeydown}
					onblur={onTextareaBlur}
					{@attach autoHResize}
				></textarea>

				<CommentGifDrawer
					show={showGifSearch}
					{gifQuery}
					{onGifQueryInput}
					{gifResults}
					{gifLoading}
					{gifError}
					{fetchGifs}
					{selectGif}
				/>

				<CommentKaomojiDrawer
					show={showKaomojiSearch}
					{kaomojiMood}
					{onKaomojiMoodInput}
					{kaomojiResults}
					{kaomojiSuggestions}
					{kaomojiLoading}
					{kaomojiTotal}
					{kaomojiError}
					{fetchKaomojis}
					{selectKaomoji}
					{applyKaomojiMoodSuggestion}
				/>

				<div class="border-t-2 border-primary/15 bg-white/40 p-3 flex flex-col gap-2">
					<span class="text-base font-semibold text-primary/70 select-none">Live Preview</span>
					<div class="w-full min-h-12 overflow-hidden">
						{#if previewHtml}
							<Comment content={previewHtml} />
						{:else}
							<p class="text-sm text-dark/40 italic select-none">
								Nothing to preview yet. Start typing... <span aria-hidden="true">ヽ(ヅ)ノ</span>
							</p>
						{/if}
					</div>
				</div>

				<CommentAutocompletePopover
					{commandState}
					{mentionState}
					{popoverTop}
					{pickCommandItem}
					{applyKaomojiSuggestion}
					{pickMention}
				/>
			</div>

			<CommentComposerToolbar
				{showMarkdownHelp}
				{showKaomojiSearch}
				{showGifSearch}
				{onToggleHelp}
				{onHeader}
				{onBold}
				{onItalic}
				{onCode}
				{onToggleKaomoji}
				{onToggleGif}
			/>
		</div>

		<CommentMarkdownHelp open={showMarkdownHelp} close={onCloseMarkdownHelp} />

		{#if replyTo}
			<div
				class="flex items-center justify-between gap-4 rounded-xl border-2 border-dark/20 bg-primary-20 px-3 py-2"
			>
				<span class="text-sm text-dark/80">
					Replying to {replyTo.display_name ?? replyTo.username ?? 'comment'}
				</span>
				<button type="button" class="text-sm text-dark/70 hover:text-dark" onclick={onCancelReply}>
					cancel
				</button>
			</div>
		{/if}

		<div class="ml-auto mb-4 w-fit duo-btn duo-blue">
			<button
				class="fill-white"
				type="button"
				disabled={sending || value.length < 1}
				onclick={onSubmit}
			>
				<svg class="w-6 -scale-x-100 -translate-y-0.5 inline-block" viewbox="0 0 24 24">
					<g>
						<path
							fill-rule="evenodd"
							clip-rule="evenodd"
							d="M3.3572 3.23397C3.66645 2.97447 4.1014 2.92638 4.45988 3.11204L20.7851 11.567C21.1426 11.7522 21.3542 12.1337 21.322 12.5351C21.2898 12.9364 21.02 13.2793 20.6375 13.405L13.7827 15.6586L10.373 22.0179C10.1828 22.3728 9.79826 22.5789 9.39743 22.541C8.9966 22.503 8.65762 22.2284 8.53735 21.8441L3.04564 4.29872C2.92505 3.91345 3.04794 3.49346 3.3572 3.23397ZM5.67123 5.99173L9.73507 18.9752L12.2091 14.361C12.3304 14.1347 12.5341 13.9637 12.7781 13.8835L17.7518 12.2484L5.67123 5.99173Z"
						></path>
					</g>
				</svg>
				Send
			</button>
		</div>
	</div>
</div>

<style lang="postcss">
	@reference "../../../app.css";

	.comment-input {
		overflow-wrap: anywhere;
		word-break: break-word;
		white-space: pre-wrap;
	}
</style>
