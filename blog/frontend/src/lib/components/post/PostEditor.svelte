<script>
	import { goto } from '$app/navigation';
	import { auth, authState } from '$lib/auth/user.svelte.js';
	import { arraysEqualIgnoreOrder, nowToDate } from '$lib/utils';
	import { untrack } from 'svelte';
	import { useDebounce } from '$lib/utils/debounce';
	import PostCard from '../home/PostCard.svelte';
	import ContentDebounceEditor from './ContentDebounceEditor.svelte';
	import MediaDictionaryController from './MediaDictionaryController.svelte';
	import PostSection from './PostSection.svelte';
	import SeriesController from './SeriesController.svelte';
	import RelatedPostsController from './RelatedPostsController.svelte';
	import EditorToolbar from '../editor/EditorToolbar.svelte';
	import EditorCoverUploader from '../editor/EditorCoverUploader.svelte';
	import PBody from '../shell/PBody.svelte';

	const mediaSyntax = /\@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]/g;
	const glbSyntax = /:::app\s+glb-demo\s+([^\s]+)\s*/g;
	const lottieAppSyntax = /:::app\s+lottie\s+([^\s]+)\s*/g;

	// temporary usage only
	const ignored = ['.glb'];
	//

	let { mode = 'create', data, series: initialSeries = [], isOwner = true } = $props();

	let mediaDictionary = $state({});
	let searchMedia = $state(async (keyword) => {});
	let forceContent = $state((content) => {});
	let isOnline = $state((keyword) => false);
	let isOffline = $state((keyword) => false);
	let getNewMedia = $state((keyword) => {});
	let clearNewMedia = $state((keywords) => {});

	const updateMediaDictionary = (newDictionary) => {
		mediaDictionary = { ...newDictionary };
	};

	let editingData = $state({
		id: '',
		title: '',
		_slugStatus: {},
		slug: '',
		tags: '',
		categories: [],
		series: [],
		seriesSlug: '',
		excerpt: '',
		date: nowToDate(),
		content: '',
		draft: '',
		coverUrl: '',
		videoShortName: '',
		ogImageSeconds: 0,
		coverMediaType: '',
		pendingSeriesId: null,
		relatedPosts: [],
		author: {
			username: authState.user?.username,
			displayName: authState.user?.displayName,
			avatarUrl: authState.user?.avatarUrl
		}
	});

	let editor = $state({
		toggled: false,
		view: 'private',
		status: '',
		isCritical: false,
		isPublishing: false
	});

	let coverUploaderOpen = $state(false);

	let renderedText = $state('');

	let forDraft = $derived(mode === 'create' || (mode === 'edit' && editor.view === 'private'));

	const collectMediaKeys = (texts) => [
		...new Set(
			texts.flatMap((text) => [
				...[...text.matchAll(mediaSyntax)].map((match) => match[1]),
				...[...text.matchAll(glbSyntax)].map((match) => match[1]),
				...[...text.matchAll(lottieAppSyntax)].map((match) => match[1])
			])
		)
	];

	if (untrack(() => mode) === 'edit' && untrack(() => data) !== undefined) {
		const d = untrack(() => data);
		let {
			id,
			title,
			slug,
			series,
			seriesSlug,
			content,
			draft,
			excerpt,
			tags,
			coverUrl,
			mediumShortNames,
			mediumUrls
		} = d;
		editingData.id = id;
		editingData.title = title;
		editingData.slug = slug;
		editingData.excerpt = excerpt;
		editingData.tags = tags.join(' ');
		editingData.content = content;
		editingData.draft = draft;
		editingData.series = series;
		editingData.seriesSlug = seriesSlug ?? '';
		editingData.coverUrl = coverUrl;
		editingData.videoShortName = d.videoShortName ?? '';
		editingData.ogImageSeconds = d.ogImageSeconds ?? 0;
		editingData.coverMediaType = d.cover_media_type;

		// Load existing related posts asynchronously
		fetch(`/api/posts/id/${id}/related`, {
			headers: { Authorization: auth() }
		})
			.then((r) => (r.ok ? r.json() : { posts: [] }))
			.then(({ posts }) => {
				editingData.relatedPosts = posts ?? [];
			})
			.catch(() => {});

		editor.view = 'public';
	} else if (untrack(() => mode) === 'create') {
		editingData.series = untrack(() => initialSeries);
	}

	let slugDebounce = useDebounce(async (slug) => {
		if (slug.length < 5) return;

		if (!(slug in editingData._slugStatus)) {
			if (slug === data?.slug) {
				editingData._slugStatus[slug] = 'ready';
			} else {
				editingData._slugStatus[slug] = 'pending';
				const res = await fetch('/api/posts/check?slug=' + slug, {
					method: 'GET',
					headers: {
						'Content-Type': 'application/json'
					}
				});

				if (res.ok) {
					const { exists } = await res.json();
					editingData._slugStatus[slug] = !exists ? 'ready' : 'used';
				} else {
					delete editingData._slugStatus[slug];
				}
			}
		}
	}, 300);

	$effect(() => {
		forceContent(forDraft ? editingData.draft : editingData.content);
	});

	$effect(() => {
		slugDebounce.update(editingData.slug);
	});

	let timeout;
	$effect(() => {
		if (editor.status === '') return;

		editor.status;
		clearTimeout(timeout);
		timeout = setTimeout(() => {
			editor.status = '';
		}, 2000);
	});

	const toggleVersion = () => {
		if (editor.view === 'public') {
			forceContent(editingData.draft);
			editor.view = 'private';
			editor.toggled = true;
		} else {
			forceContent(editingData.content);
			editor.view = 'public';
		}
	};

	const newPost = async () => {
		const { title, slug, excerpt, draft } = editingData;
		const tags = editingData.tags
			.trim()
			.split(' ')
			.filter((tag) => tag !== '');

		const keys = collectMediaKeys([draft]).filter((key) => !isOnline(key));

		const missing = keys.filter((key) => !isOffline(key));

		if (missing.length > 0) {
			editor.isCritical = true;
			editor.status = `[${missing}] is/are missing`;
			console.error('Missing keys detected: ', missing);
			return;
		}

		const formData = new FormData();
		formData.append(
			'post_data',
			new Blob(
				[
					JSON.stringify({
						title,
						slug,
						excerpt,
						tags,
						content: draft,
						categories: [],
						number_of_files: keys.length,
						...(editingData.videoShortName && { video_short_name: editingData.videoShortName }),
						...(editingData.ogImageSeconds > 0 && { og_image_seconds: editingData.ogImageSeconds })
					})
				],
				{ type: 'application/json' }
			)
		);

		for (let index = 0; index < keys.length; index++) {
			const mediaItem = getNewMedia(keys[index]);
			formData.append(`file_${index + 1}`, mediaItem.file, mediaItem.file.name);
			formData.append(`short_name_${index + 1}`, keys[index]);
		}

		editor.isCritical = false;
		editor.status = '';

		const res = await fetch('/api/posts/new', {
			method: 'POST',
			headers: { Authorization: auth() },
			body: formData
		});

		if (res.ok) {
			editor.isCritical = false;
			editor.status = 'OK!';
			const { id } = await res.json();
			if (editingData.pendingSeriesId) {
				await fetch(`/api/series/id/${editingData.pendingSeriesId}?post_id=${id}`, {
					method: 'PATCH',
					headers: { Authorization: auth() }
				});
			}
			goto(`/dashboard/posts/id/${id}`);
		} else {
			editor.isCritical = true;
			editor.status = await res.text();
		}
	};

	const updatePost = async () => {
		const formData = new FormData();
		const postData = {
			number_of_files: 0
		};
		let offlineKeys = [];
		const contentChanged = editingData.draft !== data.draft;
		const keys = collectMediaKeys([editingData.content, editingData.draft]);

		offlineKeys = keys.filter((key) => {
			if (ignored.some((ext) => key.endsWith(ext))) {
				return false;
			}
			return !isOnline(key);
		});

		console.log(keys, offlineKeys);

		const missing = offlineKeys.filter((key) => !isOffline(key));

		if (missing.length > 0) {
			editor.isCritical = true;
			editor.status = `[${missing}] is/are missing`;
			console.error('Missing keys detected: ', missing);
			return;
		}

		postData.number_of_files = offlineKeys.length;

		for (let index = 0; index < offlineKeys.length; index++) {
			const data = getNewMedia(offlineKeys[index]);
			formData.append(`file_${index + 1}`, data.file, data.file.name);
			formData.append(`short_name_${index + 1}`, offlineKeys[index]);
		}

		if (editingData.title !== data.title) {
			postData.title = editingData.title;
		}

		if (editingData.slug !== data.slug) {
			postData.slug = editingData.slug;
		}

		const tags = editingData.tags
			.trim()
			.split(' ')
			.filter((tag) => tag !== '');
		if (!arraysEqualIgnoreOrder(tags, data.tags)) {
			postData.tags = tags;
		}

		if (editingData.excerpt !== data.excerpt) {
			postData.excerpt = editingData.excerpt;
		}

		if (contentChanged || offlineKeys.length > 0) {
			postData.draft = editingData.draft;
			postData.content = editingData.content;
		}

		if (editingData.videoShortName !== (data.videoShortName ?? '')) {
			postData.video_short_name = editingData.videoShortName || null;
		}
		if (editingData.ogImageSeconds !== (data.ogImageSeconds ?? 0)) {
			postData.og_image_seconds = editingData.ogImageSeconds;
		}

		formData.append(
			'post_data',
			new Blob([JSON.stringify(postData)], { type: 'application/json' })
		);

		const res = await fetch('/api/posts/id/' + data.id, {
			method: 'PATCH',
			headers: { Authorization: auth() },
			body: formData
		});

		if (res.ok) {
			editor.isCritical = false;
			editor.status = 'OK!';
			clearNewMedia(offlineKeys);
		} else {
			editor.isCritical = true;
			editor.status = await res.text();
		}
	};

	const publishPost = async () => {
		if (editor.isPublishing) return;
		editor.isPublishing = true;

		const res = await fetch('/api/posts/id/' + data.id, {
			method: 'POST',
			headers: { Authorization: auth() }
		});

		editor.isPublishing = false;
		if (res.ok) {
			editor.isCritical = false;
			editor.status = 'Published!';
		} else {
			editor.isCritical = true;
			editor.status = await res.text();
		}
	};
</script>

<EditorCoverUploader
	show={coverUploaderOpen}
	apiPath={`/api/posts/id/${editingData.id}/cover`}
	onclose={() => (coverUploaderOpen = false)}
	onuploaded={(newCoverUrl) => (editingData.coverUrl = newCoverUrl)}
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
				author={{
					name: authState.user.displayName,
					slug: authState.user.username
				}}
				tags={editingData.tags.split(' ').filter((tag) => tag !== '')}
				src={editingData.coverUrl}
				coverMediaType={editingData.coverMediaType ||
					(editingData.videoShortName ? 'video/mp4' : undefined)}
				stats={{
					views: '#',
					likes: '#',
					comments_count: '#'
				}}
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
				ontogglevision={toggleVersion}
				onsubmit={newPost}
				onchange={updatePost}
				onpublish={publishPost}
			/>
			<div class="flex not-lg:flex-col gap-2 w-full h-full p-2 pt-1">
				<div class="flex grow gap-2">
					<div class="max-h-80 p-2 pr-1 w-1/3 bg-primary/40 rounded-lg">
						<div class="full space-y-2 pr-0.75 custom-scrollbar overflow-y-scroll">
							<div class="flex not-sm:flex-col">
								<label class="inline-block min-w-11" for="title">Title:</label>
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
								<label class="inline-block min-w-11" for="slug">Slug:</label>
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
								<label class="inline-block" for="slug">Tags:</label>
								<textarea
									class="p-1 outline-none bg-white rounded-sm resize-none custom-scrollbar"
									autocorrect="off"
									autocomplete="off"
									rows="2"
									readonly={!isOwner}
									bind:value={editingData.tags}></textarea>
							</div>
							<div class="flex flex-col">
								<label class="inline-block" for="slug">Excerpt:</label>
								<textarea
									class="p-1 outline-none bg-white rounded-sm resize-none custom-scrollbar"
									autocorrect="off"
									autocomplete="off"
									rows="5"
									readonly={!isOwner}
									bind:value={editingData.excerpt}></textarea>
							</div>
							<div class="flex flex-col">
								<label for="video-short-name">Cover video short name:</label>
								<input
									id="video-short-name"
									class="px-1 min-w-0 bg-white rounded-sm"
									bind:value={editingData.videoShortName}
									readonly={!isOwner}
									placeholder="e.g. my-demo-video"
								/>
							</div>
							<div class="flex flex-col">
								<label for="og-image-seconds">OG image seconds:</label>
								<input
									id="og-image-seconds"
									type="number"
									class="px-1 min-w-0 bg-white rounded-sm"
									bind:value={editingData.ogImageSeconds}
									readonly={!isOwner}
									min="0"
								/>
							</div>
							<SeriesController
								postId={mode === 'edit' ? editingData.id : null}
								bind:series={editingData.series}
								bind:seriesSlug={editingData.seriesSlug}
								onSelect={(id) => {
									editingData.pendingSeriesId = id;
								}}
							/>
							{#if mode === 'edit'}
								<RelatedPostsController
									postId={editingData.id}
									bind:relatedPosts={editingData.relatedPosts}
								/>
							{/if}
						</div>
					</div>
					<ContentDebounceEditor
						class="max-h-80 grow bg-primary/40 p-2 rounded-lg"
						delay="500"
						onUpdateRendered={(_renderedText) => (renderedText = _renderedText)}
						onUpdateDraft={(content) => (editingData.draft = content)}
						disabled={editor.view === 'public'}
						{mediaSyntax}
						{mediaDictionary}
						{searchMedia}
						{forDraft}
						registerForceContent={(fn) => {
							forceContent = fn;
						}}
					/>
				</div>
				<MediaDictionaryController
					class="flex max-h-80 gap-2 not-lg:h-40 overflow-hidden"
					registerMediaCheck={({ isOnline: _isOnline, isOffline: _isOffline }) => {
						isOnline = _isOnline;
						isOffline = _isOffline;
					}}
					registerGetMedia={(fn) => (getNewMedia = fn)}
					{updateMediaDictionary}
					registerSearch={(fn) => (searchMedia = fn)}
					registerClearNewMedia={(fn) => (clearNewMedia = fn)}
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
