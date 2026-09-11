<script>
	import { browser } from '$app/environment';
	import { preventDefault } from '$lib/utils';
	import PostContent from '../post/PostContent.svelte';
	import ContentDebounceEditor from '../post/ContentDebounceEditor.svelte';
	import MediaDictionaryController from '../post/MediaDictionaryController.svelte';
	import EditorToolbar from './EditorToolbar.svelte';
	import EditorFeedback from './EditorFeedback.svelte';
	import EditorCoverUploader from './EditorCoverUploader.svelte';
	import CreateCoverField from './CreateCoverField.svelte';
	import CreateDraftPublishPrompt from './CreateDraftPublishPrompt.svelte';
	import FullPreviewModal from './FullPreviewModal.svelte';

	let {
		vm,
		kind = 'post',
		coverApiPath = '',
		titleLabel = 'Title',
		excerptRows = 5,
		extraFields
	} = $props();

	const entry = $derived(vm.entry);
	const ui = $derived(vm.ui);
	const mode = vm.mode;
	const isOwner = vm.isOwner;

	let coverUploaderOpen = $state(false);
	let fullPreviewOpen = $state(false);
	let coverDropFile = $state(null);
	// Set by the content editor's debounce: the preview pane is behind the
	// textarea. Makes the 500 ms lag legible instead of looking like dropped
	// input.
	let previewPending = $state(false);

	// A resizable source/preview split, persisted across sessions. Dragging is
	// plain pointer-event math — no dependency pulled in for something this
	// small.
	const SPLIT_KEY = 'editor-split-ratio';
	let splitRatio = $state(browser ? Number(localStorage.getItem(SPLIT_KEY) ?? 0.5) || 0.5 : 0.5);
	let paneEl = $state();
	let dragging = $state(false);

	function startDrag(e) {
		dragging = true;
		e.preventDefault();
	}

	function onDrag(e) {
		if (!dragging || !paneEl) return;
		const rect = paneEl.getBoundingClientRect();
		const next = (e.clientX - rect.left) / rect.width;
		splitRatio = Math.min(0.8, Math.max(0.2, next));
	}

	function stopDrag() {
		if (!dragging) return;
		dragging = false;
		if (browser) localStorage.setItem(SPLIT_KEY, String(splitRatio));
	}

	// Missing-media keys detected by the content editor's own debounce; shown
	// next to the media panel rather than only as a toast that disappears.
	//
	// The keys are kept in state and the missing set is *derived* from the
	// media dictionary. `search` resolves referenced short names asynchronously
	// (key -> url via /api/media/s/:key), so the dictionary for existing media
	// is empty on load; deriving instead of snapshotting means a key stops
	// reading as missing the moment its lookup lands, instead of pinning a
	// stale "missing" verdict from the debounce tick that detected it.
	let bodyKeys = $state([]);
	function onKeysChanged(keys) {
		bodyKeys = keys;
		vm.media.search(keys);
	}
	const missingKeys = $derived(
		bodyKeys.filter(
			(key) => !key.endsWith('.glb') && !vm.media.isOffline(key) && !vm.media.isOnline(key)
		)
	);

	// Live, not a message: a count of what the editor already knows, so it sits
	// beside the media library heading and never flashes. A *blocked save* does
	// raise a sticky banner (the vm's `reportMissingMedia`) because a silently
	// refused save is worse than either; once the references resolve that banner
	// has served its purpose, and its exit is resolution.
	$effect(() => {
		if (missingKeys.length === 0) vm.feedback.dismiss('missing-media');
	});
</script>

<svelte:window onpointermove={onDrag} onpointerup={stopDrag} />

<EditorCoverUploader
	show={coverUploaderOpen}
	apiPath={coverApiPath}
	initialFile={coverDropFile}
	onclose={() => {
		coverUploaderOpen = false;
		coverDropFile = null;
	}}
	onuploaded={vm.onCoverUploaded}
/>

<CreateDraftPublishPrompt
	show={ui.createPromptOpen}
	{kind}
	busy={ui.createPromptBusy}
	error={ui.createPromptError}
	onconfirm={() => vm.finishCreateFlow(true)}
	oncancel={() => vm.finishCreateFlow(false)}
/>

<FullPreviewModal
	show={fullPreviewOpen}
	id={entry.id || null}
	title={entry.title}
	tags={entry.tags.split(' ').filter((t) => t !== '')}
	date={entry.date}
	content={vm.renderedText}
	relatedPosts={entry.relatedPosts}
	onclose={() => (fullPreviewOpen = false)}
/>

<article class="flex flex-col gap-4 pb-10">
	<EditorToolbar {vm} {titleLabel} />
	<EditorFeedback {vm} />

	<div class="flex not-xl:flex-col gap-4">
		<div class="flex flex-col grow min-w-0 gap-4">
			<div
				bind:this={paneEl}
				class="relative flex flex-col xl:grid gap-0 rounded-xl overflow-hidden border-2 border-dark/10 bg-white xl:h-[68vh]"
				style={`grid-template-columns: ${splitRatio}fr auto ${1 - splitRatio}fr`}
			>
				<div class="min-w-0 h-96 xl:h-full overflow-hidden bg-white p-4">
					<ContentDebounceEditor
						class="full p-1"
						delay={500}
						bind:value={entry.body}
						bind:pending={previewPending}
						disabled={!isOwner}
						mediaDictionary={vm.media.dictionary}
						onRenderedUpdate={(html) => (vm.renderedText = html)}
						{onKeysChanged}
					/>
				</div>
				<button
					type="button"
					aria-label="Resize source/preview split"
					class="hidden xl:block w-1.5 shrink-0 cursor-col-resize bg-dark/25 hover:bg-dark/50 transition-colors"
					onpointerdown={startDrag}
				></button>
				<div class="min-w-0 h-96 xl:h-full bg-white flex flex-col overflow-hidden">
					<div
						class="flex shrink-0 items-center justify-between gap-2 px-4 py-2 border-b border-dark/10"
					>
						<span class="text-sm font-semibold uppercase tracking-wide text-dark/50">
							Preview
							{#if previewPending}
								<span class="ml-1 font-normal normal-case tracking-normal text-dark/40">
									· updating
								</span>
							{/if}
						</span>
						<div class="duo-btn" data-duo-color="blue">
							<button onclick={() => (fullPreviewOpen = true)}>Full page</button>
						</div>
					</div>
					<div class="min-w-0 grow overflow-y-auto custom-scrollbar m-4">
						<PostContent title={entry.title} content={vm.renderedText} hideBackButton />
					</div>
				</div>
			</div>

			<div class="rounded-xl border-2 border-dark/10 bg-white p-4">
				<div class="mb-3 flex flex-wrap items-baseline justify-between gap-2">
					<h3 class="text-sm font-semibold uppercase tracking-wide text-dark/50">
						Media library
					</h3>
					<!--
						Live counter rather than the flashing red paragraph this
						replaced: it only counts tokens the content editor has
						*settled*, so it cannot flicker mid-word, and it never
						pushes the layout around. Names are on the tooltip; a
						blocked save spells them out in the banner.
					-->
					{#if bodyKeys.length > 0}
						<span class="text-xs text-dark/50" title={missingKeys.join(', ')}>
							{#if missingKeys.length > 0}
								<span class="font-medium text-accent-red">
									{missingKeys.length} missing
								</span>
							{:else}
								{bodyKeys.length} of {bodyKeys.length} resolved
							{/if}
						</span>
					{/if}
				</div>
				<MediaDictionaryController
					class="flex max-h-60 gap-3 not-xl:h-44 overflow-hidden"
					media={vm.media}
				/>
			</div>

			<section class="rounded-xl border-2 border-dark/10 bg-white p-4 space-y-4">
				<h3 class="mb-3 text-sm font-semibold uppercase tracking-wide text-dark/50">More</h3>
				{@render extraFields?.()}
			</section>
		</div>

		<aside class="w-full xl:w-80 shrink-0 flex flex-col gap-4">
			<section class="rounded-xl border-2 border-dark/10 bg-white p-4">
				<h3 class="mb-3 text-sm font-semibold uppercase tracking-wide text-dark/50">Cover</h3>
				<button
					type="button"
					class="block w-full aspect-video rounded-xl overflow-hidden border-2 border-dashed border-dark/30 bg-background/30"
					class:cursor-not-allowed={!isOwner}
					aria-disabled={!isOwner}
					onclick={() => {
						if (mode === 'edit' && isOwner) coverUploaderOpen = true;
					}}
					ondrop={(e) => {
						if (!isOwner) return;
						e.preventDefault();
						const file = e.dataTransfer.files[0];
						if (!file) return;
						if (mode === 'edit') {
							coverDropFile = file;
							coverUploaderOpen = true;
						} else {
							vm.setCreateCover(file);
						}
					}}
					ondragover={preventDefault}
				>
					{#if entry.coverUrl}
						{#if entry.coverMediaType?.startsWith('video/')}
							<video class="full object-cover" src={entry.coverUrl} muted></video>
						{:else}
							<img class="full object-cover" src={entry.coverUrl} alt="cover" />
						{/if}
					{:else}
						<span class="grid place-items-center full text-sm text-dark/50">
							Click or drag a cover here
						</span>
					{/if}
				</button>
				{#if mode === 'create'}
					<CreateCoverField
						file={ui.createCoverFile}
						error={ui.createCoverError}
						ogImageSeconds={entry.ogImageSeconds}
						{isOwner}
						onselect={vm.setCreateCover}
						onsecondschange={(seconds) => (entry.ogImageSeconds = seconds)}
					/>
				{:else if entry.coverMediaType?.startsWith('video/')}
					<div class="mt-3 flex flex-col gap-1">
						<label class="text-sm font-medium text-dark/60" for="og-image-seconds">
							Thumbnail seconds
						</label>
						<input
							id="og-image-seconds"
							type="number"
							class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white"
							bind:value={entry.ogImageSeconds}
							readonly={!isOwner}
							min="0"
						/>
					</div>
				{/if}
			</section>

			<section class="rounded-xl border-2 border-dark/10 bg-white p-4">
				<h3 class="mb-3 text-sm font-semibold uppercase tracking-wide text-dark/50">Details</h3>
				<div class="flex flex-col gap-3">
					<div class="flex flex-col gap-1">
						<label class="text-sm font-medium text-dark/60" for="tags">Tags</label>
						<textarea
							id="tags"
							class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors resize-none custom-scrollbar focus:bg-primary focus:text-white"
							autocorrect="off"
							autocomplete="off"
							rows="2"
							readonly={!isOwner}
							bind:value={entry.tags}></textarea>
					</div>
					<div class="flex flex-col gap-1">
						<label class="text-sm font-medium text-dark/60" for="editor-excerpt">Excerpt</label>
						<textarea
							id="editor-excerpt"
							class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 transition-colors resize-none custom-scrollbar focus:bg-primary focus:text-white
								{ui.fieldErrors.excerpt ? 'border-accent-red' : 'border-dark'}"
							autocorrect="off"
							autocomplete="off"
							rows={excerptRows}
							readonly={!isOwner}
							oninput={() => vm.clearFieldError('excerpt')}
							bind:value={entry.excerpt}></textarea>
						{#if ui.fieldErrors.excerpt}
							<p class="text-sm font-medium text-accent-red">{ui.fieldErrors.excerpt}</p>
						{/if}
					</div>
				</div>
			</section>
		</aside>
	</div>
</article>
