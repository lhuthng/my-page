<script>
	import { authState } from '$lib/auth/user.svelte.js';
	import PostCard from '../home/PostCard.svelte';
	import PostSection from '../post/PostSection.svelte';
	import ContentDebounceEditor from '../post/ContentDebounceEditor.svelte';
	import MediaDictionaryController from '../post/MediaDictionaryController.svelte';
	import EditorToolbar from './EditorToolbar.svelte';
	import EditorCoverUploader from './EditorCoverUploader.svelte';
	import CreateCoverField from './CreateCoverField.svelte';
	import CreateDraftPublishPrompt from './CreateDraftPublishPrompt.svelte';

	let {
		mode = 'create',
		isOwner = true,
		kind = 'post',
		routePrefix = '/posts',
		dashboardPrefix = '/dashboard/posts/id',
		coverApiPath = '',
		titleLabel = 'Title',
		excerptRows = 5,
		editingData,
		editor,
		renderedText = '',
		forDraft = true,
		mediaSyntax,
		mediaDictionary,
		searchMedia,
		onToggleVersion = () => {},
		onSubmit = () => {},
		onChange = () => {},
		onPublish = () => {},
		onCreateCoverSelect = () => {},
		onCreateCoverSecondsChange = () => {},
		onCoverUploaded = () => {},
		onRenderedUpdate = () => {},
		onDraftUpdate = () => {},
		onForceContent = () => {},
		onMediaCheck = () => {},
		onGetMedia = () => {},
		onMediaDictionaryUpdate = () => {},
		onMediaSearch = () => {},
		onClearNewMedia = () => {},
		onPromptConfirm = () => {},
		onPromptCancel = () => {},
		extraFields
	} = $props();

	let coverUploaderOpen = $state(false);
</script>

<EditorCoverUploader
	show={coverUploaderOpen}
	apiPath={coverApiPath}
	onclose={() => (coverUploaderOpen = false)}
	onuploaded={onCoverUploaded}
/>

<CreateDraftPublishPrompt
	show={editor.createPromptOpen}
	{kind}
	busy={editor.createPromptBusy}
	error={editor.createPromptError}
	onconfirm={onPromptConfirm}
	oncancel={onPromptCancel}
/>

<article class="relative flex flex-col gap-4 pb-4 *:drop-shadow-xl">
	<div class="flex w-full">
		<div class="p-2 rounded-xl bg-white mx-auto w-120">
			<PostCard
				id={editingData.id}
				dashboardMode={true}
				title={editingData.title === '' ? '<Empty>' : editingData.title}
				slug={editingData.slug}
				excerpt={editingData.excerpt === '' ? '<Empty>' : editingData.excerpt}
				author={{ name: authState.user.displayName, slug: authState.user.username }}
				tags={editingData.tags.split(' ').filter((tag) => tag !== '')}
				src={editingData.coverUrl}
				coverMediaType={editingData.coverMediaType ||
					(editingData.videoShortName ? 'video/mp4' : undefined)}
				stats={{ views: '#', likes: '#', comments_count: '#' }}
				{routePrefix}
				{dashboardPrefix}
				onclick={() => {
					if (mode !== 'edit' || !isOwner) return;
					coverUploaderOpen = true;
				}}
			>
				<div
					class="absolute top-0 full z-20 grid place-items-center border-4 border-dashed border-accent-green rounded-lg opacity-0 bg-accent-green/40 hover:opacity-100 hover:scale-105 transition-all duration-100"
				></div>
			</PostCard>
		</div>
	</div>

	<PostSection
		title={editingData.title}
		tags={editingData.tags.split(' ').filter((tag) => tag !== '')}
		date={editingData.date}
		content={renderedText}
		author={editingData.author}
	/>

	<div id="padding"></div>
	<div
		class="fixed z-10 top-full left-1/2 -translate-x-1/2 w-full max-w-400 transition-transform duration-100 -translate-y-14"
		class:-translate-y-full={editor.toggled}
	>
		<div
			class="absolute z-9 left-1/2 top-1/2 -translate-1/2 w-[calc(100%+6px)] h-[calc(100%+6px)] bg-dark/20 rounded-t-xl"
		></div>
		<div
			class="relative z-10 flex flex-col items-center bg-white border-2 border-dark not-sm:text-sm rounded-t-xl"
		>
			<EditorToolbar
				bind:toggled={editor.toggled}
				view={editor.view}
				status={editor.status}
				bind:isCritical={editor.isCritical}
				isPublishing={editor.isPublishing}
				{mode}
				{isOwner}
				ontoggle={() => (editor.toggled = !editor.toggled)}
				ontogglevision={onToggleVersion}
				onsubmit={onSubmit}
				onchange={onChange}
				onpublish={onPublish}
			/>
			<div class="flex not-lg:flex-col gap-2 w-full h-full p-2 pt-1">
				<div class="flex grow gap-2">
					<div class="max-h-80 p-2 pr-1 w-1/3 bg-primary/40 rounded-lg">
						<div class="full space-y-2 pr-0.75 custom-scrollbar overflow-y-scroll">
							<div class="flex not-sm:flex-col">
								<label class="inline-block min-w-14" for="title">{titleLabel}:</label>
								<input
									id="title"
									class="grow px-1 min-w-0 bg-white rounded-sm"
									class:bg-transparent!={!isOwner}
									bind:value={editingData.title}
									autocomplete="off"
									readonly={!isOwner}
									required
								/>
							</div>
							<div class="flex not-sm:flex-col">
								<label class="inline-block min-w-14" for="slug">Slug:</label>
								<input
									id="slug"
									class="grow px-1 min-w-0 bg-white rounded-sm"
									class:bg-red-200!={editingData._slugStatus[editingData.slug] === 'used'}
									class:bg-yellow-200!={editingData._slugStatus[editingData.slug] === 'pending'}
									class:bg-green-200!={editingData._slugStatus[editingData.slug] === 'ready'}
									bind:value={editingData.slug}
									autocomplete="off"
									readonly={!isOwner}
									required
								/>
							</div>
							<div class="flex flex-col">
								<label for="tags">Tags:</label>
								<textarea
									id="tags"
									class="p-1 outline-none bg-white rounded-sm resize-none custom-scrollbar"
									autocorrect="off"
									autocomplete="off"
									rows="2"
									readonly={!isOwner}
									bind:value={editingData.tags}></textarea>
							</div>
							<div class="flex flex-col">
								<label for="excerpt">Excerpt:</label>
								<textarea
									id="excerpt"
									class="p-1 outline-none bg-white rounded-sm resize-none custom-scrollbar"
									autocorrect="off"
									autocomplete="off"
									rows={excerptRows}
									readonly={!isOwner}
									bind:value={editingData.excerpt}></textarea>
							</div>
							{#if mode === 'create'}
								<CreateCoverField
									file={editor.createCoverFile}
									error={editor.createCoverError}
									ogImageSeconds={editingData.ogImageSeconds}
									{isOwner}
									onselect={onCreateCoverSelect}
									onsecondschange={onCreateCoverSecondsChange}
								/>
							{:else if mode === 'edit'}
								<div class="flex flex-col">
									<label for="og-image-seconds">Thumbnail seconds:</label>
									<input
										id="og-image-seconds"
										type="number"
										class="px-1 min-w-0 bg-white rounded-sm"
										bind:value={editingData.ogImageSeconds}
										readonly={!isOwner}
										min="0"
									/>
								</div>
							{/if}
							{@render extraFields?.()}
						</div>
					</div>
					<ContentDebounceEditor
						class="max-h-80 grow bg-primary/40 p-2 rounded-lg"
						delay="500"
						onUpdateRendered={onRenderedUpdate}
						onUpdateDraft={onDraftUpdate}
						disabled={editor.view === 'public'}
						{mediaSyntax}
						{mediaDictionary}
						{searchMedia}
						{forDraft}
						registerForceContent={onForceContent}
					/>
				</div>
				<MediaDictionaryController
					class="flex max-h-80 gap-2 not-lg:h-40 overflow-hidden"
					registerMediaCheck={onMediaCheck}
					registerGetMedia={onGetMedia}
					updateMediaDictionary={onMediaDictionaryUpdate}
					registerSearch={onMediaSearch}
					registerClearNewMedia={onClearNewMedia}
				/>
			</div>
		</div>
	</div>
</article>

<style lang="postcss">
	@reference "../../../app.css";

	#padding {
		@apply h-80;
	}
</style>
