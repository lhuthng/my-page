import { goto, beforeNavigate } from '$app/navigation';
import { browser } from '$app/environment';
import { auth, authState } from '$lib/auth/user.svelte.js';
import { useDebounce } from '$lib/utils/debounce';
import { createEntryState, loadEntryState, refreshBaseline } from '../model/state.js';
import { buildPatch, isPatchEmpty } from '../model/diff.js';
import { applyDemoTypeTransition, validateDemoFields } from '../model/demo.js';
import { collectMediaKeys } from '../media/references.js';
import { createMediaDictionary } from '../media/dictionary.svelte.js';
import {
	checkSlugAvailable,
	createEntry,
	patchEntry,
	publishEntry,
	deleteEntryDraft
} from '../data/editor-api.js';
import { saveLocalDraft, loadLocalDraft, clearLocalDraft } from '../data/local-draft.js';
import { createUploadController } from '../controllers/upload-controller.js';
import {
	selectCreateCover,
	appendCreateCover,
	revokeCreateCoverPreview
} from '$lib/components/editor/createCover.js';
import {
	openCreatedDraftPrompt,
	finishCreatedDraftPrompt
} from '$lib/components/editor/draftCreation.js';

// Inline media tokens with these extensions are never round-tripped through
// the offline/online media dictionary (see the original `PostEditor.svelte`
// comment: "temporary usage only").
const IGNORED_MEDIA_EXTENSIONS = ['.glb'];

const NOTICE_MS = 2200;

function isIgnoredMediaKey(key) {
	return IGNORED_MEDIA_EXTENSIONS.some((ext) => key.endsWith(ext));
}

/**
 * The single `$state` owner for the post, project, and game editors.
 *
 * Replaces the pattern where `PostEditor.svelte`/`ProjectEditor.svelte` held
 * `editingData`/`editor` directly, wired seven function-valued `$state` slots
 * for children to fill in, ran an effect cycle to keep the draft/published
 * bodies in sync across a component boundary, and diffed against a `data`
 * prop that was never refreshed after a save.
 *
 * @param {object} args
 * @param {'create'|'edit'} args.mode
 * @param {'post'|'project'|'game'} args.kind
 * @param {object} [args.data] loaded entry, required in edit mode
 * @param {object[]} [args.initialSeries]
 * @param {boolean} [args.isOwner]
 * @param {typeof fetch} [args.fetchImpl]
 */
export function createEditorViewModel({
	mode,
	kind,
	data,
	initialSeries = [],
	isOwner = true,
	fetchImpl = fetch
}) {
	const initial =
		mode === 'edit' && data
			? loadEntryState(data, kind)
			: { entry: createEntryState(kind, initialSeries), baseline: null };

	let entry = $state(initial.entry);
	let baseline = $state(initial.baseline);

	entry.author = {
		username: authState.user?.username,
		displayName: authState.user?.displayName,
		avatarUrl: authState.user?.avatarUrl
	};

	let renderedText = $state('');

	let ui = $state({
		view: mode === 'edit' ? 'published' : 'draft',
		toggled: false,
		slugStatus: {},
		notice: '',
		noticeCritical: false,
		progress: '',
		save: { status: 'idle', error: '' }, // idle | saving | saved | error | conflict
		isPublishing: false,
		createCoverFile: undefined,
		createCoverError: '',
		createPromptOpen: false,
		createPromptBusy: false,
		createPromptError: '',
		createdEntryId: null,
		...(kind === 'project' || kind === 'game'
			? { demoZip: undefined, demoZipName: '', demoZipError: '' }
			: {})
	});

	const media = createMediaDictionary({ fetchImpl });
	const upload = createUploadController({ authHeader: auth, fetchImpl, onProgress: setProgress });

	const forDraft = $derived(mode === 'create' || (mode === 'edit' && ui.view === 'draft'));
	const activeBodyKey = $derived(forDraft ? 'draft' : 'content');

	const pendingPatch = $derived(
		baseline ? buildPatch({ baseline, current: entry, hasNewMedia: false, kind }) : {}
	);
	// In create mode there is no baseline to diff against yet — "dirty" means
	// the user has typed anything into a field that would be lost by leaving.
	const hasCreateContent = $derived(
		mode === 'create' &&
			(entry.title !== '' ||
				entry.slug !== '' ||
				entry.excerpt !== '' ||
				entry.tags !== '' ||
				entry.bodies.draft !== '')
	);
	const isDirty = $derived(mode === 'edit' ? !isPatchEmpty(pendingPatch) : hasCreateContent);

	// ---- notices -----------------------------------------------------------
	// A single timer that clears itself, rather than an `$effect` that reads
	// and writes `ui.notice` in the same run — that pattern raced with upload
	// progress messages sharing the same field (see `upload`'s `onProgress`).
	let noticeTimer;
	function notify(message, { critical = false, autoClearMs = NOTICE_MS } = {}) {
		ui.notice = message;
		ui.noticeCritical = critical;
		clearTimeout(noticeTimer);
		if (autoClearMs > 0) {
			noticeTimer = setTimeout(() => {
				ui.notice = '';
				ui.noticeCritical = false;
			}, autoClearMs);
		}
	}

	function setProgress(message) {
		ui.progress = message;
	}

	// ---- slug availability --------------------------------------------------
	const slugDebounce = useDebounce(async (slug) => {
		if (slug.length < 5) return;
		if (slug in ui.slugStatus) return;

		if (mode === 'edit' && slug === data?.slug) {
			ui.slugStatus[slug] = 'ready';
			return;
		}

		ui.slugStatus[slug] = 'pending';
		try {
			const available = await checkSlugAvailable(kind, slug, fetchImpl);
			ui.slugStatus[slug] = available ? 'ready' : 'used';
		} catch {
			delete ui.slugStatus[slug];
		}
	}, 300);

	$effect(() => {
		slugDebounce.update(entry.slug);
	});

	// ---- unsaved-changes guard -----------------------------------------------
	// `finishCreateFlow` navigates to the new entry's edit page right after a
	// successful create, while the create form is still "dirty" — that
	// navigation is the point of a successful save, not something to confirm.
	let suppressGuard = false;
	beforeNavigate((navigation) => {
		if (suppressGuard || !isDirty) return;
		const proceed = confirm('You have unsaved changes. Leave without saving?');
		if (!proceed) navigation.cancel();
	});

	if (browser) {
		$effect(() => {
			const handler = (e) => {
				if (!isDirty) return;
				e.preventDefault();
				e.returnValue = '';
			};
			window.addEventListener('beforeunload', handler);
			return () => window.removeEventListener('beforeunload', handler);
		});
	}

	// ---- local autosave + recovery -------------------------------------------
	const localDraftKeyId = mode === 'edit' ? entry.id : '';
	let localDraft = $state(browser ? loadLocalDraft(kind, localDraftKeyId) : null);
	// A stored draft is only worth surfacing if its body actually differs from
	// what's currently loaded — otherwise every visit would show the bar.
	const localDraftAvailable = $derived(
		!!localDraft && localDraft.draft !== undefined && localDraft.draft !== entry.bodies.draft
	);

	const autosaveDebounce = useDebounce(() => {
		if (!browser) return;
		saveLocalDraft(kind, localDraftKeyId, {
			title: entry.title,
			slug: entry.slug,
			tags: entry.tags,
			excerpt: entry.excerpt,
			draft: entry.bodies.draft
		});
	}, 800);

	$effect(() => {
		// Re-run whenever any of these change; body is the field actually worth
		// protecting against loss.
		void entry.title;
		void entry.tags;
		void entry.excerpt;
		autosaveDebounce.update(entry.bodies.draft);
	});

	function recoverLocalDraft() {
		if (!localDraft) return;
		if (localDraft.title !== undefined) entry.title = localDraft.title;
		if (localDraft.slug !== undefined) entry.slug = localDraft.slug;
		if (localDraft.tags !== undefined) entry.tags = localDraft.tags;
		if (localDraft.excerpt !== undefined) entry.excerpt = localDraft.excerpt;
		if (localDraft.draft !== undefined) entry.bodies.draft = localDraft.draft;
		localDraft = null;
	}

	function discardLocalDraft() {
		clearLocalDraft(kind, localDraftKeyId);
		localDraft = null;
	}

	// ---- version toggle -------------------------------------------------------
	function toggleVersion() {
		ui.view = ui.view === 'published' ? 'draft' : 'published';
		if (ui.view === 'draft') ui.toggled = true;
	}

	// ---- cover ------------------------------------------------------------
	function setCreateCover(file) {
		const previousUrl = entry.coverUrl;
		const next = selectCreateCover(file);
		ui.createCoverFile = next.file;
		ui.createCoverError = next.error;
		entry.coverUrl = next.previewUrl;
		entry.coverMediaType = next.mediaType;
		entry.ogImageSeconds = next.ogImageSeconds;
		revokeCreateCoverPreview(previousUrl);
	}

	function onCoverUploaded({ url, ogImageSeconds, fileType }) {
		entry.coverUrl = url;
		entry.coverMediaType = fileType;
		if (fileType?.startsWith('video/')) entry.ogImageSeconds = ogImageSeconds;
	}

	// ---- project/game demo type ---------------------------------------------------
	function setDemoType(nextType) {
		if (kind === 'post' || nextType === entry.demoType) return;
		const patch = applyDemoTypeTransition(nextType);
		Object.assign(entry, {
			demoType: patch.demoType,
			...('demoUrl' in patch ? { demoUrl: patch.demoUrl } : {})
		});
		if ('demoZip' in patch) {
			ui.demoZip = patch.demoZip;
			ui.demoZipName = patch.demoZipName;
			ui.demoZipError = patch.demoZipError;
		}
	}

	function setDemoZip(file) {
		ui.demoZipError = '';
		if (!file) return;
		const isJsDos = entry.demoType === 'jsdos';
		if (!file.name.toLowerCase().endsWith(isJsDos ? '.jsdos' : '.zip')) {
			ui.demoZipError = isJsDos
				? 'Only .jsdos bundles are allowed.'
				: 'Only zip archives are allowed.';
			return;
		}
		if (file.size > 524288000) {
			ui.demoZipError = isJsDos
				? 'The js-dos bundle cannot exceed 500 MB.'
				: 'The game ZIP cannot exceed 500 MiB.';
			return;
		}
		ui.demoZip = file;
		ui.demoZipName = file.name;
	}

	// ---- media validation -----------------------------------------------------
	function collectOfflineKeys(texts) {
		const allKeys = collectMediaKeys(texts);
		const offlineKeys = allKeys.filter((k) => !isIgnoredMediaKey(k) && media.isOffline(k));
		const missing = allKeys.filter(
			(k) => !isIgnoredMediaKey(k) && !media.isOffline(k) && !media.isOnline(k)
		);
		return { allKeys, offlineKeys, missing };
	}

	function appendInlineFiles(formData, keys) {
		keys.forEach((key, index) => {
			const item = media.getNew(key);
			formData.append(`file_${index + 1}`, item.file, item.file.name);
			formData.append(`short_name_${index + 1}`, key);
		});
	}

	// ---- create / save / publish (post) ---------------------------------------
	async function submitPost() {
		if (ui.save.status === 'saving') return;
		const { offlineKeys, missing } = collectOfflineKeys([entry.bodies.draft]);
		if (missing.length > 0) {
			notify(`[${missing}] is/are missing`, { critical: true, autoClearMs: 0 });
			return;
		}

		const tags = splitTags(entry.tags);
		const formData = new FormData();
		formData.append(
			'post_data',
			new Blob(
				[
					JSON.stringify({
						title: entry.title,
						slug: entry.slug,
						excerpt: entry.excerpt,
						tags,
						content: entry.bodies.draft,
						categories: [],
						number_of_files: offlineKeys.length
					})
				],
				{ type: 'application/json' }
			)
		);
		appendInlineFiles(formData, offlineKeys);
		appendCreateCover(formData, ui.createCoverFile, entry.ogImageSeconds);

		try {
			ui.save.status = 'saving';
			const { id } = await createEntry('post', formData, auth(), fetchImpl);
			ui.save.status = 'saved';
			notify('OK!');
			if (entry.pendingSeriesId) {
				await fetchImpl(`/api/series/id/${entry.pendingSeriesId}?post_id=${id}`, {
					method: 'PATCH',
					headers: { Authorization: auth() }
				});
			}
			discardLocalDraft();
			openCreatedDraftPrompt(ui, id);
		} catch (error) {
			ui.save.status = 'error';
			notify(error.message ?? 'Save failed.', { critical: true, autoClearMs: 0 });
		}
	}

	async function savePost() {
		if (ui.save.status === 'saving') return;
		const bothBodies = [entry.bodies.content, entry.bodies.draft];
		const { offlineKeys, missing } = collectOfflineKeys(bothBodies);
		if (missing.length > 0) {
			notify(`[${missing}] is/are missing`, { critical: true, autoClearMs: 0 });
			return;
		}

		const patch = buildPatch({ baseline, current: entry, hasNewMedia: offlineKeys.length > 0 });
		if (isPatchEmpty(patch) && offlineKeys.length === 0) {
			notify('Nothing to save.');
			return;
		}

		const formData = new FormData();
		formData.append(
			'post_data',
			new Blob(
				[
					JSON.stringify({
						...patch,
						number_of_files: offlineKeys.length,
						expected_updated_at: baseline.updatedAt
					})
				],
				{ type: 'application/json' }
			)
		);
		appendInlineFiles(formData, offlineKeys);

		try {
			ui.save.status = 'saving';
			const response = await patchEntry('post', entry.id, formData, auth(), fetchImpl);
			baseline = refreshBaseline(baseline, entry, { updatedAt: response.updated_at });
			ui.save.status = 'saved';
			notify('OK!');
			media.clearNew(offlineKeys);
			discardLocalDraft();
		} catch (error) {
			if (error.conflict) {
				ui.save.status = 'conflict';
				ui.save.error = error.currentUpdatedAt;
				notify('Someone else saved this in the meantime.', { critical: true, autoClearMs: 0 });
				return;
			}
			ui.save.status = 'error';
			notify(error.message ?? 'Save failed.', { critical: true, autoClearMs: 0 });
		}
	}

	async function publish() {
		if (ui.isPublishing) return;
		ui.isPublishing = true;
		try {
			await publishEntry(kind, entry.id, auth(), fetchImpl);
			notify('Published!');
		} catch (error) {
			notify(error.message ?? 'Publish failed.', { critical: true, autoClearMs: 0 });
		} finally {
			ui.isPublishing = false;
		}
	}

	// ---- create / save (project) -----------------------------------------------
	function normalizedLinks() {
		return entry.links
			.map((link) => ({ label: link.label.trim(), url: link.url.trim() }))
			.filter((link) => link.label && link.url);
	}

	async function prepareV86ForSubmit(sourceProjectId) {
		return upload.prepareV86Artifact({
			file: ui.demoZip,
			sourceProjectId,
			systemVersionId: entry.v86SystemVersionId,
			expectedArtifactRevision: sourceProjectId ? entry.v86ArtifactRevision : 0,
			manifest: entry.v86Manifest
		});
	}

	async function submitProject() {
		if (ui.save.status === 'saving') return;
		const demoValidation = validateDemoFields({
			demoType: entry.demoType,
			demoUrl: entry.demoUrl,
			zip: ui.demoZip,
			mode: 'create',
			delegateGameId: entry.delegateGameId
		});
		if (!demoValidation.valid) {
			notify(demoValidation.error, { critical: true, autoClearMs: 0 });
			return;
		}

		const { offlineKeys, missing } = collectOfflineKeys([entry.bodies.draft]);
		if (missing.length > 0) {
			notify(`[${missing}] is/are missing`, { critical: true, autoClearMs: 0 });
			return;
		}

		const payload = {
			title: entry.title,
			slug: entry.slug,
			excerpt: entry.excerpt,
			tags: splitTags(entry.tags),
			content: entry.bodies.draft,
			links: normalizedLinks(),
			number_of_files: offlineKeys.length,
			demo_type: entry.demoType,
			demo_width: entry.demoWidth,
			demo_height: entry.demoHeight
		};
		if (entry.demoType === 'game') {
			payload.delegate_game_id = Number(entry.delegateGameId);
			payload.inherit_thumbnail = entry.inheritThumbnail;
			payload.inherit_tags = entry.inheritTags;
		}
		if (!['none', 'html5', 'webgl', 'game'].includes(entry.demoType)) {
			payload.demo_url = entry.demoUrl;
		}

		const formData = new FormData();
		formData.append(
			'project_data',
			new Blob([JSON.stringify(payload)], { type: 'application/json' })
		);
		appendCreateCover(formData, ui.createCoverFile, entry.ogImageSeconds);
		if (ui.demoZip) {
			formData.append('demo_zip', ui.demoZip, ui.demoZip.name);
		}
		appendInlineFiles(formData, offlineKeys);

		try {
			ui.save.status = 'saving';
			const { id } = await createEntry('project', formData, auth(), fetchImpl);
			ui.save.status = 'saved';
			notify('OK!');
			discardLocalDraft();
			openCreatedDraftPrompt(ui, id);
		} catch (error) {
			ui.save.status = 'error';
			notify(error.message ?? 'Save failed.', { critical: true, autoClearMs: 0 });
		}
	}

	async function saveProject() {
		if (ui.save.status === 'saving') return;
		const demoValidation = validateDemoFields({
			demoType: entry.demoType,
			demoUrl: entry.demoUrl,
			zip: ui.demoZip,
			mode: 'edit',
			previousDemoType: data?.demoType,
			delegateGameId: entry.delegateGameId
		});
		if (!demoValidation.valid) {
			notify(demoValidation.error, { critical: true, autoClearMs: 0 });
			return;
		}

		const bothBodies = [entry.bodies.content, entry.bodies.draft];
		const { offlineKeys, missing } = collectOfflineKeys(bothBodies);
		if (missing.length > 0) {
			notify(`[${missing}] is/are missing`, { critical: true, autoClearMs: 0 });
			return;
		}

		const patch = buildPatch({
			baseline,
			current: entry,
			hasNewMedia: offlineKeys.length > 0,
			kind: 'project'
		});

		// Checked before adding the bookkeeping fields below — `number_of_files`
		// and `expected_updated_at` are always present, so testing emptiness
		// after adding them would never be true and this early-return would be
		// dead code.
		if (isPatchEmpty(patch) && offlineKeys.length === 0 && !ui.demoZip) {
			notify('Nothing to save.');
			return;
		}

		patch.number_of_files = offlineKeys.length;
		patch.expected_updated_at = baseline.updatedAt;

		const formData = new FormData();
		formData.append(
			'project_data',
			new Blob([JSON.stringify(patch)], { type: 'application/json' })
		);
		if (ui.demoZip) {
			formData.append('demo_zip', ui.demoZip, ui.demoZip.name);
		}
		appendInlineFiles(formData, offlineKeys);

		try {
			ui.save.status = 'saving';
			const response = await patchEntry('project', entry.id, formData, auth(), fetchImpl);
			baseline = refreshBaseline(baseline, entry, { updatedAt: response.updated_at });
			ui.save.status = 'saved';
			notify('OK!');
			media.clearNew(offlineKeys);
			ui.demoZip = undefined;
			ui.demoZipName = '';
			discardLocalDraft();
		} catch (error) {
			if (error.conflict) {
				ui.save.status = 'conflict';
				ui.save.error = error.currentUpdatedAt;
				notify('Someone else saved this in the meantime.', { critical: true, autoClearMs: 0 });
				return;
			}
			ui.save.status = 'error';
			notify(error.message ?? 'Save failed.', { critical: true, autoClearMs: 0 });
		}
	}

	// ---- create / save (game) -----------------------------------------------
	async function submitGame() {
		if (ui.save.status === 'saving') return;
		const demoValidation = validateDemoFields({
			demoType: entry.demoType,
			demoUrl: entry.demoUrl,
			zip: ui.demoZip,
			mode: 'create',
			v86SystemVersionId: entry.v86SystemVersionId,
			v86Manifest: entry.v86Manifest,
			kindLabel: 'games'
		});
		if (!demoValidation.valid) {
			notify(demoValidation.error, { critical: true, autoClearMs: 0 });
			return;
		}

		const { offlineKeys, missing } = collectOfflineKeys([entry.bodies.draft]);
		if (missing.length > 0) {
			notify(`[${missing}] is/are missing`, { critical: true, autoClearMs: 0 });
			return;
		}

		let v86UploadId;
		if (entry.demoType === 'v86') {
			try {
				v86UploadId = await prepareV86ForSubmit(undefined);
			} catch (error) {
				notify(error?.message ?? 'v86 package build failed.', { critical: true, autoClearMs: 0 });
				return;
			}
		}

		const payload = {
			title: entry.title,
			slug: entry.slug,
			excerpt: entry.excerpt,
			tags: splitTags(entry.tags),
			content: entry.bodies.draft,
			number_of_files: offlineKeys.length,
			launcher_type: entry.demoType,
			demo_width: entry.demoWidth,
			demo_height: entry.demoHeight,
			instruction: entry.instruction ?? '',
			cheatcode: entry.cheatcode ?? '',
			story: entry.story ?? '',
			related_games: (entry.relatedGames ?? [])
				.filter((link) => link.id !== '' && link.id != null)
				.map((link) => ({ id: Number(link.id), title: link.title, slug: link.slug }))
		};
		if (v86UploadId) payload.v86_upload_id = v86UploadId;
		if (!['html5', 'webgl', 'jsdos', 'v86'].includes(entry.demoType)) {
			payload.demo_url = entry.demoUrl;
		}

		const formData = new FormData();
		formData.append('game_data', new Blob([JSON.stringify(payload)], { type: 'application/json' }));
		appendCreateCover(formData, ui.createCoverFile, entry.ogImageSeconds);
		if (ui.demoZip && entry.demoType !== 'jsdos' && entry.demoType !== 'v86') {
			formData.append('demo_zip', ui.demoZip, ui.demoZip.name);
		}
		appendInlineFiles(formData, offlineKeys);

		try {
			ui.save.status = 'saving';
			const { id } = await createEntry('game', formData, auth(), fetchImpl);
			ui.save.status = 'saved';
			notify('OK!');
			try {
				if (entry.demoType === 'jsdos' && ui.demoZip) {
					await upload.uploadJsDosBundle('game', id, ui.demoZip);
				}
			} catch (error) {
				await deleteEntryDraft('game', id, auth(), fetchImpl);
				notify(error?.message ?? 'Game upload failed.', { critical: true, autoClearMs: 0 });
				return;
			}
			discardLocalDraft();
			openCreatedDraftPrompt(ui, id);
		} catch (error) {
			ui.save.status = 'error';
			notify(error.message ?? 'Save failed.', { critical: true, autoClearMs: 0 });
		}
	}

	async function saveGame() {
		if (ui.save.status === 'saving') return;
		const demoValidation = validateDemoFields({
			demoType: entry.demoType,
			demoUrl: entry.demoUrl,
			zip: ui.demoZip,
			mode: 'edit',
			previousDemoType: data?.demoType,
			v86SystemVersionId: entry.v86SystemVersionId,
			v86Manifest: entry.v86Manifest,
			kindLabel: 'games'
		});
		if (!demoValidation.valid) {
			notify(demoValidation.error, { critical: true, autoClearMs: 0 });
			return;
		}

		const v86Changed =
			entry.demoType === 'v86' &&
			(ui.demoZip ||
				entry.v86SystemVersionId !== data?.v86SystemVersionId?.toString() ||
				entry.v86Manifest !== (data?.v86Manifest ?? '') ||
				(data?.demoType ?? 'html5') !== 'v86');

		let v86UploadId;
		if (v86Changed) {
			try {
				v86UploadId = await prepareV86ForSubmit(data?.demoType === 'v86' ? data.id : undefined);
			} catch (error) {
				notify(error?.message ?? 'v86 package build failed.', { critical: true, autoClearMs: 0 });
				return;
			}
		}

		const bothBodies = [entry.bodies.content, entry.bodies.draft];
		const { offlineKeys, missing } = collectOfflineKeys(bothBodies);
		if (missing.length > 0) {
			notify(`[${missing}] is/are missing`, { critical: true, autoClearMs: 0 });
			return;
		}

		const patch = buildPatch({
			baseline,
			current: entry,
			hasNewMedia: offlineKeys.length > 0,
			kind: 'game'
		});

		if (isPatchEmpty(patch) && offlineKeys.length === 0 && !ui.demoZip && !v86UploadId) {
			notify('Nothing to save.');
			return;
		}

		if (v86UploadId) patch.v86_upload_id = v86UploadId;
		patch.number_of_files = offlineKeys.length;
		patch.expected_updated_at = baseline.updatedAt;

		const formData = new FormData();
		formData.append('game_data', new Blob([JSON.stringify(patch)], { type: 'application/json' }));
		if (ui.demoZip && entry.demoType !== 'jsdos' && entry.demoType !== 'v86') {
			formData.append('demo_zip', ui.demoZip, ui.demoZip.name);
		}
		appendInlineFiles(formData, offlineKeys);

		try {
			ui.save.status = 'saving';
			const response = await patchEntry('game', entry.id, formData, auth(), fetchImpl);
			baseline = refreshBaseline(baseline, entry, { updatedAt: response.updated_at });
			ui.save.status = 'saved';
			notify('OK!');
			media.clearNew(offlineKeys);
			try {
				if (entry.demoType === 'jsdos' && ui.demoZip) {
					await upload.uploadJsDosBundle(kind, entry.id, ui.demoZip);
				}
			} catch (error) {
				notify(error?.message ?? 'Game upload failed.', { critical: true, autoClearMs: 0 });
				return;
			}
			ui.demoZip = undefined;
			ui.demoZipName = '';
			discardLocalDraft();
		} catch (error) {
			if (error.conflict) {
				ui.save.status = 'conflict';
				ui.save.error = error.currentUpdatedAt;
				notify('Someone else saved this in the meantime.', { critical: true, autoClearMs: 0 });
				return;
			}
			ui.save.status = 'error';
			notify(error.message ?? 'Save failed.', { critical: true, autoClearMs: 0 });
		}
	}

	// ---- create-draft-publish prompt -------------------------------------------
	async function finishCreateFlow(publishNow) {
		// This is about to navigate away from a "dirty" create form on purpose —
		// suppress the unsaved-changes guard for exactly this navigation.
		suppressGuard = true;
		try {
			await finishCreatedDraftPrompt({
				editor: ui,
				publishNow,
				publishPath: (id) => `/api/${kind}s/id/${id}`,
				gotoPath: (id) => `/dashboard/${kind}s/id/${id}`,
				fetchImpl,
				authHeader: auth(),
				goto
			});
		} finally {
			suppressGuard = false;
		}
	}

	// ---- reload after a conflict ------------------------------------------------
	function acceptRemoteVersion() {
		// A full reload re-runs the page's server load function, which already
		// does the correct field mapping, media-token decoding, and URL fixup —
		// duplicating that client-side for an in-place refresh isn't worth it
		// for a path that only exists because two saves collided.
		if (!browser) return;
		suppressGuard = true;
		window.location.reload();
	}

	function overwriteRemoteVersion() {
		if (!baseline) return;
		baseline = { ...baseline, updatedAt: ui.save.error };
		ui.save.status = 'idle';
		ui.save.error = '';
		if (kind === 'post') return savePost();
		if (kind === 'game') return saveGame();
		return saveProject();
	}

	function destroy() {
		clearTimeout(noticeTimer);
		slugDebounce.destroy();
		autosaveDebounce.destroy();
		media.destroy();
		if (mode === 'create') revokeCreateCoverPreview(entry.coverUrl);
	}

	return {
		mode,
		kind,
		isOwner,
		get entry() {
			return entry;
		},
		get baseline() {
			return baseline;
		},
		get ui() {
			return ui;
		},
		media,
		get renderedText() {
			return renderedText;
		},
		set renderedText(value) {
			renderedText = value;
		},
		get forDraft() {
			return forDraft;
		},
		get activeBodyKey() {
			return activeBodyKey;
		},
		get isDirty() {
			return isDirty;
		},
		get localDraftAvailable() {
			return localDraftAvailable;
		},
		recoverLocalDraft,
		discardLocalDraft,
		toggleVersion,
		setCreateCover,
		onCoverUploaded,
		setDemoType,
		setDemoZip,
		notify,
		setProgress,
		submit: kind === 'post' ? submitPost : kind === 'game' ? submitGame : submitProject,
		save: kind === 'post' ? savePost : kind === 'game' ? saveGame : saveProject,
		publish,
		finishCreateFlow,
		acceptRemoteVersion,
		overwriteRemoteVersion,
		destroy
	};
}

function splitTags(tagsText) {
	return (tagsText ?? '')
		.trim()
		.split(/\s+/)
		.filter((t) => t !== '');
}
