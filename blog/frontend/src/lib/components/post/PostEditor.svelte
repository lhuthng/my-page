<script>
	import { goto } from '$app/navigation';
	import { auth, authState } from '$lib/auth/user.svelte.js';
	import { arraysEqualIgnoreOrder, nowToDate } from '$lib/utils';
	import { untrack } from 'svelte';
	import { useDebounce } from '$lib/utils/debounce';
	import SeriesController from './SeriesController.svelte';
	import RelatedPostsController from './RelatedPostsController.svelte';
	import PostEditorShell from '../editor/PostEditorShell.svelte';
	import { appendCreateCover, selectCreateCover } from '../editor/createCover.js';
	import { finishCreatedDraftPrompt, openCreatedDraftPrompt } from '../editor/draftCreation.js';

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
		isPublishing: false,
		createCoverFile: undefined,
		createCoverError: '',
		createPromptOpen: false,
		createPromptBusy: false,
		createPromptError: '',
		createdEntryId: null
	});

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
			mediumUrls,
			relatedPosts
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
		editingData.relatedPosts = relatedPosts ?? [];

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

	const setCreateCover = (file) => {
		const next = selectCreateCover(file);
		editor.createCoverFile = next.file;
		editor.createCoverError = next.error;
		editingData.coverUrl = next.previewUrl;
		editingData.coverMediaType = next.mediaType;
		editingData.ogImageSeconds = next.ogImageSeconds;
	};

	const finishCreateFlow = async (publishNow) => {
		await finishCreatedDraftPrompt({
			editor,
			publishNow,
			publishPath: (id) => `/api/posts/id/${id}`,
			gotoPath: (id) => `/dashboard/posts/id/${id}`,
			fetchImpl: fetch,
			authHeader: auth(),
			goto
		});
	};

	const newPost = async () => {
		const { title, slug, excerpt, draft } = editingData;
		const tags = editingData.tags
			.trim()
			.split(' ')
			.filter((tag) => tag !== '');

		const keys = collectMediaKeys([draft]);

		const offlineKeys = keys.filter((key) => {
			if (ignored.some((ext) => key.endsWith(ext))) {
				return false;
			}
			return isOffline(key);
		});

		const missing = keys.filter((key) => {
			if (ignored.some((ext) => key.endsWith(ext))) {
				return false;
			}
			return !isOffline(key) && !isOnline(key);
		});

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
						number_of_files: offlineKeys.length
					})
				],
				{ type: 'application/json' }
			)
		);

		for (let index = 0; index < offlineKeys.length; index++) {
			const mediaItem = getNewMedia(offlineKeys[index]);
			formData.append(`file_${index + 1}`, mediaItem.file, mediaItem.file.name);
			formData.append(`short_name_${index + 1}`, offlineKeys[index]);
		}
		appendCreateCover(formData, editor.createCoverFile, editingData.ogImageSeconds);

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
			openCreatedDraftPrompt(editor, id);
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
			return isOffline(key);
		});

		const missing = keys.filter((key) => {
			if (ignored.some((ext) => key.endsWith(ext))) {
				return false;
			}
			return !isOffline(key) && !isOnline(key);
		});

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

{#snippet extraFields()}
	<SeriesController
		postId={mode === 'edit' ? editingData.id : null}
		bind:series={editingData.series}
		bind:seriesSlug={editingData.seriesSlug}
		onSelect={(id) => {
			editingData.pendingSeriesId = id;
		}}
	/>
	{#if mode === 'edit'}
		<RelatedPostsController postId={editingData.id} bind:relatedPosts={editingData.relatedPosts} />
	{/if}
{/snippet}

<PostEditorShell
	{mode}
	{isOwner}
	kind="post"
	routePrefix="/posts"
	dashboardPrefix="/dashboard/posts/id"
	coverApiPath={`/api/posts/id/${editingData.id}/cover`}
	titleLabel="Title"
	excerptRows={5}
	{editingData}
	{editor}
	{renderedText}
	{forDraft}
	{mediaSyntax}
	{mediaDictionary}
	{searchMedia}
	onToggleVersion={toggleVersion}
	onSubmit={newPost}
	onChange={updatePost}
	onPublish={publishPost}
	onCreateCoverSelect={setCreateCover}
	onCreateCoverSecondsChange={(seconds) => (editingData.ogImageSeconds = seconds)}
	onCoverUploaded={({ url, ogImageSeconds, fileType }) => {
		editingData.coverUrl = url;
		editingData.coverMediaType = fileType;
		if (fileType?.startsWith('video/')) {
			editingData.ogImageSeconds = ogImageSeconds;
		}
	}}
	onRenderedUpdate={(_renderedText) => (renderedText = _renderedText)}
	onDraftUpdate={(content) => (editingData.draft = content)}
	onForceContent={(fn) => {
		forceContent = fn;
	}}
	onMediaCheck={({ isOnline: _isOnline, isOffline: _isOffline }) => {
		isOnline = _isOnline;
		isOffline = _isOffline;
	}}
	onGetMedia={(fn) => (getNewMedia = fn)}
	onMediaDictionaryUpdate={updateMediaDictionary}
	onMediaSearch={(fn) => (searchMedia = fn)}
	onClearNewMedia={(fn) => (clearNewMedia = fn)}
	onPromptConfirm={() => finishCreateFlow(true)}
	onPromptCancel={() => finishCreateFlow(false)}
	{extraFields}
/>
