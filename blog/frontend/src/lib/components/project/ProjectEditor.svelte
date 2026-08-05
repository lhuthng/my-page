<script>
	import { goto } from '$app/navigation';
	import { auth, authState } from '$lib/auth/user.svelte.js';
	import { arraysEqualIgnoreOrder, nowToDate, preventDefault } from '$lib/utils';
	import { untrack } from 'svelte';
	import { useDebounce } from '$lib/utils/debounce';
	import PostEditorShell from '../editor/PostEditorShell.svelte';
	import { appendCreateCover, selectCreateCover } from '../editor/createCover.js';
	import { finishCreatedDraftPrompt, openCreatedDraftPrompt } from '../editor/draftCreation.js';

	const mediaSyntax = /\@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]/g;
	const lottieAppSyntax = /:::app\s+lottie\s+([^\s]+)\s*/g;
	const demoTypes = [
		{ value: 'none', label: 'No Demo', disabled: false },
		{ value: 'html5', label: 'HTML5', disabled: false },
		{ value: 'embed', label: 'Embed', disabled: false },
		{ value: 'webgl', label: 'WebGL', disabled: false },
		{ value: 'jsdos', label: 'js-dos', disabled: false },
		{ value: 'v86', label: 'v86', disabled: false },
		{ value: 'download', label: 'Download', disabled: false },
		{ value: 'video', label: 'Video', disabled: false }
	];

	let { mode = 'create', data, isOwner = true, v86Systems = [] } = $props();

	let mediaDictionary = $state({});
	let searchMedia = $state(async () => {});
	let forceContent = $state(() => {});
	let isOnline = $state(() => false);
	let isOffline = $state(() => false);
	let getNewMedia = $state(() => {});
	let clearNewMedia = $state(() => {});

	const updateMediaDictionary = (newDictionary) => {
		mediaDictionary = { ...newDictionary };
	};

	let editingData = $state({
		id: '',
		postId: '',
		title: '',
		_slugStatus: {},
		slug: '',
		tags: '',
		excerpt: '',
		date: nowToDate(),
		content: '',
		draft: '',
		coverUrl: '',
		videoShortName: '',
		ogImageSeconds: 0,
		coverMediaType: '',
		demoType: 'html5',
		demoWidth: '100%',
		demoHeight: '520px',
		demoUrl: '',
		v86SystemVersionId: '',
		v86Manifest: '',
		v86ArtifactRevision: 0,
		links: [{ label: 'GitHub', url: '' }],
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
		createdEntryId: null,
		demoZip: undefined,
		demoZipName: '',
		demoZipError: ''
	});

	let renderedText = $state('');
	let forDraft = $derived(mode === 'create' || (mode === 'edit' && editor.view === 'private'));

	if (untrack(() => mode) === 'edit' && untrack(() => data) !== undefined) {
		const d = untrack(() => data);
		editingData.id = d.id;
		editingData.postId = d.postId;
		editingData.title = d.title;
		editingData.slug = d.slug;
		editingData.excerpt = d.excerpt;
		editingData.tags = d.tags.join(' ');
		editingData.content = d.content;
		editingData.draft = d.draft;
		editingData.coverUrl = d.coverUrl;
		editingData.videoShortName = d.videoShortName ?? '';
		editingData.ogImageSeconds = d.ogImageSeconds ?? 0;
		editingData.coverMediaType = d.cover_media_type;
		editingData.demoType = d.demoType ?? 'html5';
		editingData.demoWidth = d.demoWidth ?? '100%';
		editingData.demoHeight = d.demoHeight ?? '520px';
		editingData.demoUrl = d.rawDemoUrl ?? '';
		editingData.v86SystemVersionId = d.v86SystemVersionId?.toString() ?? '';
		editingData.v86Manifest = d.v86Manifest ?? '';
		editingData.v86ArtifactRevision = d.v86ArtifactRevision ?? 0;
		editingData.links = d.links?.length ? d.links : [{ label: 'GitHub', url: '' }];
		editor.view = 'public';
	}

	let slugDebounce = useDebounce(async (slug) => {
		if (slug.length < 5) return;
		if (slug === data?.slug) {
			editingData._slugStatus[slug] = 'ready';
			return;
		}
		if (!(slug in editingData._slugStatus)) {
			editingData._slugStatus[slug] = 'pending';
			const res = await fetch('/api/projects/check?slug=' + slug);
			if (res.ok) {
				const { exists } = await res.json();
				editingData._slugStatus[slug] = !exists ? 'ready' : 'used';
			} else {
				delete editingData._slugStatus[slug];
			}
		}
	}, 300);

	$effect(() => {
		forceContent(forDraft ? editingData.draft : editingData.content);
	});

	$effect(() => {
		slugDebounce.update(editingData.slug);
	});

	let statusTimeout;
	$effect(() => {
		if (editor.status === '') return;
		clearTimeout(statusTimeout);
		statusTimeout = setTimeout(() => {
			editor.status = '';
		}, 2200);
	});

	$effect(() => {
		if (
			editingData.demoType === 'embed' ||
			editingData.demoType === 'download' ||
			editingData.demoType === 'video'
		) {
			editor.demoZip = undefined;
			editor.demoZipName = '';
			editor.demoZipError = '';
		} else if (editingData.demoType === 'none') {
			editor.demoZip = undefined;
			editor.demoZipName = '';
			editor.demoZipError = '';
			editingData.demoUrl = '';
		} else {
			editingData.demoUrl = '';
		}
	});

	const normalizedLinks = () =>
		editingData.links
			.map((link) => ({ label: link.label.trim(), url: link.url.trim() }))
			.filter((link) => link.label && link.url);

	const collectOfflineKeys = (texts) => {
		const keys = texts.flatMap((text) => [
			...[...text.matchAll(mediaSyntax)].map((match) => match[1]),
			...[...text.matchAll(lottieAppSyntax)].map((match) => match[1])
		]);
		return [...new Set(keys.filter((key) => !isOnline(key)))];
	};

	const appendInlineFiles = (formData, keys) => {
		for (let index = 0; index < keys.length; index++) {
			const mediaItem = getNewMedia(keys[index]);
			formData.append(`file_${index + 1}`, mediaItem.file, mediaItem.file.name);
			formData.append(`short_name_${index + 1}`, keys[index]);
		}
	};

	const validateOfflineKeys = (keys) => {
		const missing = keys.filter((key) => !isOffline(key));
		if (missing.length > 0) {
			editor.isCritical = true;
			editor.status = `[${missing}] is/are missing`;
			return false;
		}
		return true;
	};

	const validateDemoFields = () => {
		const type = editingData.demoType;
		const url = editingData.demoUrl;
		const zip = editor.demoZip;

		switch (type) {
			case 'none':
				break;
			case 'html5':
			case 'webgl':
				if (mode === 'create' && !zip) {
					return {
						valid: false,
						error: `Zip file is required for ${type.toUpperCase()} projects.`
					};
				}
				break;
			case 'jsdos':
				if (mode === 'create' && !zip)
					return { valid: false, error: 'A .jsdos bundle is required for js-dos projects.' };
				break;
			case 'v86':
				if (!editingData.v86SystemVersionId) return { valid: false, error: 'Select a v86 system.' };
				if ((mode === 'create' || data?.demoType !== 'v86') && !zip)
					return { valid: false, error: 'A game ZIP is required for v86 projects.' };
				if (new TextEncoder().encode(editingData.v86Manifest).length > 65536)
					return { valid: false, error: 'The v86 manifest cannot exceed 64 KiB.' };
				break;

			case 'embed':
				if (!url) {
					return { valid: false, error: 'Demo URL is required for Embed projects.' };
				}
				break;
			case 'download':
				if (!url) {
					return { valid: false, error: 'Download URL is required.' };
				}
				break;
			case 'video':
				if (!url) {
					return { valid: false, error: 'Video URL is required.' };
				}
				break;
			default:
				return { valid: false, error: `Unsupported demo type: ${type}` };
		}
		return { valid: true };
	};

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
			publishPath: (id) => `/api/projects/id/${id}`,
			gotoPath: (id) => `/dashboard/projects/id/${id}`,
			fetchImpl: fetch,
			authHeader: auth(),
			goto
		});
	};

	const newProject = async () => {
		const demoValidation = validateDemoFields();
		if (!demoValidation.valid) {
			editor.isCritical = true;
			editor.status = demoValidation.error;
			return;
		}

		const tags = editingData.tags
			.trim()
			.split(' ')
			.filter((tag) => tag !== '');
		const offlineKeys = collectOfflineKeys([editingData.draft]);
		if (!validateOfflineKeys(offlineKeys)) return;

		let v86UploadId;
		if (editingData.demoType === 'v86') {
			try {
				v86UploadId = await prepareV86Artifact();
			} catch (error) {
				editor.isCritical = true;
				editor.status = error?.message ?? 'v86 package build failed.';
				return;
			}
		}
		const formData = new FormData();
		const projectPayload = {
			title: editingData.title,
			slug: editingData.slug,
			excerpt: editingData.excerpt,
			tags,
			content: editingData.draft,
			links: normalizedLinks(),
			number_of_files: offlineKeys.length,
			demo_type: editingData.demoType,
			demo_width: editingData.demoWidth,
			demo_height: editingData.demoHeight
		};
		if (v86UploadId) projectPayload.v86_upload_id = v86UploadId;
		if (
			editingData.demoType !== 'none' &&
			editingData.demoType !== 'html5' &&
			editingData.demoType !== 'webgl' &&
			editingData.demoType !== 'v86'
		) {
			projectPayload.demo_url = editingData.demoUrl;
		}
		formData.append(
			'project_data',
			new Blob([JSON.stringify(projectPayload)], { type: 'application/json' })
		);
		appendCreateCover(formData, editor.createCoverFile, editingData.ogImageSeconds);
		// Legacy js-dos bundles are uploaded after the project is created so the
		// chunked upload endpoint can handle large files. Do not include the file
		// in the project-creation multipart request.
		if (editor.demoZip && editingData.demoType !== 'jsdos' && editingData.demoType !== 'v86') {
			formData.append('demo_zip', editor.demoZip, editor.demoZip.name);
		}
		appendInlineFiles(formData, offlineKeys);

		const res = await fetch('/api/projects/new', {
			method: 'POST',
			headers: { Authorization: auth() },
			body: formData
		});

		if (res.ok) {
			editor.isCritical = false;
			editor.status = 'OK!';
			const { id } = await res.json();
			try {
				if (editingData.demoType === 'jsdos' && editor.demoZip) {
					await uploadJsDosBundle(id, editor.demoZip);
				}
			} catch (error) {
				await fetch(`/api/projects/id/${id}`, {
					method: 'DELETE',
					headers: { Authorization: auth() }
				}).catch(() => {});
				editor.isCritical = true;
				editor.status = error?.message ?? 'Game upload failed.';
				return;
			}
			openCreatedDraftPrompt(editor, id);
		} else {
			editor.isCritical = true;
			editor.status = await res.text();
		}
	};

	const updateProject = async () => {
		const demoValidation = validateDemoFields();
		if (!demoValidation.valid) {
			editor.isCritical = true;
			editor.status = demoValidation.error;
			return;
		}

		const v86Changed =
			editingData.demoType === 'v86' &&
			(editor.demoZip ||
				editingData.v86SystemVersionId !== data.v86SystemVersionId?.toString() ||
				editingData.v86Manifest !== (data.v86Manifest ?? '') ||
				(data.demoType ?? 'html5') !== 'v86');
		let v86UploadId;
		if (v86Changed) {
			try {
				v86UploadId = await prepareV86Artifact(data.demoType === 'v86' ? data.id : undefined);
			} catch (error) {
				editor.isCritical = true;
				editor.status = error?.message ?? 'v86 package build failed.';
				return;
			}
		}
		const formData = new FormData();
		const projectData = { number_of_files: 0 };
		let offlineKeys = [];
		const contentChanged = editingData.draft !== data.draft;

		offlineKeys = collectOfflineKeys([editingData.content, editingData.draft]);
		if (!validateOfflineKeys(offlineKeys)) return;
		projectData.number_of_files = offlineKeys.length;
		appendInlineFiles(formData, offlineKeys);

		if (editingData.title !== data.title) projectData.title = editingData.title;
		if (editingData.slug !== data.slug) projectData.slug = editingData.slug;
		if (editingData.excerpt !== data.excerpt) projectData.excerpt = editingData.excerpt;
		if (editingData.demoType !== (data.demoType ?? 'html5'))
			projectData.demo_type = editingData.demoType;
		if (v86UploadId) projectData.v86_upload_id = v86UploadId;
		if (editingData.demoWidth !== (data.demoWidth ?? '100%'))
			projectData.demo_width = editingData.demoWidth;
		if (editingData.demoHeight !== (data.demoHeight ?? '520px'))
			projectData.demo_height = editingData.demoHeight;
		if (editingData.demoType === 'none' && (data.rawDemoUrl ?? '') !== '') {
			projectData.demo_url = '';
		} else if (
			editingData.demoType !== 'none' &&
			editingData.demoType !== 'html5' &&
			editingData.demoType !== 'webgl' &&
			editingData.demoType !== 'v86' &&
			editingData.demoUrl !== (data.rawDemoUrl ?? '')
		)
			projectData.demo_url = editingData.demoUrl;

		const tags = editingData.tags
			.trim()
			.split(' ')
			.filter((tag) => tag !== '');
		if (!arraysEqualIgnoreOrder(tags, data.tags)) projectData.tags = tags;

		const links = normalizedLinks();
		if (JSON.stringify(links) !== JSON.stringify(data.links ?? [])) projectData.links = links;

		if (contentChanged || offlineKeys.length > 0) {
			projectData.draft = editingData.draft;
			projectData.content = editingData.content;
		}

		if (editingData.ogImageSeconds !== (data.ogImageSeconds ?? 0)) {
			projectData.og_image_seconds = editingData.ogImageSeconds;
		}

		formData.append(
			'project_data',
			new Blob([JSON.stringify(projectData)], { type: 'application/json' })
		);
		if (editor.demoZip && editingData.demoType !== 'jsdos' && editingData.demoType !== 'v86') {
			formData.append('demo_zip', editor.demoZip, editor.demoZip.name);
		}

		const res = await fetch('/api/projects/id/' + data.id, {
			method: 'PATCH',
			headers: { Authorization: auth() },
			body: formData
		});

		if (res.ok) {
			editor.isCritical = false;
			editor.status = 'OK!';
			clearNewMedia(offlineKeys);
			try {
				if (editingData.demoType === 'jsdos' && editor.demoZip) {
					await uploadJsDosBundle(data.id, editor.demoZip);
				}
			} catch (error) {
				editor.isCritical = true;
				editor.status = error?.message ?? 'Game upload failed.';
				return;
			}
			editor.demoZip = undefined;
			editor.demoZipName = '';
		} else {
			editor.isCritical = true;
			editor.status = await res.text();
		}
	};

	const publishProject = async () => {
		if (editor.isPublishing) return;
		editor.isPublishing = true;
		const res = await fetch('/api/projects/id/' + data.id, {
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

	const setDemoZip = (file) => {
		editor.demoZipError = '';
		if (!file) return;
		const isJsDos = editingData.demoType === 'jsdos';
		if (!file.name.toLowerCase().endsWith(isJsDos ? '.jsdos' : '.zip')) {
			editor.demoZipError = isJsDos
				? 'Only .jsdos bundles are allowed.'
				: 'Only zip archives are allowed.';
			return;
		}
		if (isJsDos && file.size > 524288000) {
			editor.demoZipError = 'The js-dos bundle cannot exceed 500 MB.';
			return;
		}
		if (!isJsDos && file.size > 524288000) {
			editor.demoZipError = 'The game ZIP cannot exceed 500 MiB.';
			return;
		}
		editor.demoZip = file;
		editor.demoZipName = file.name;
	};

	const prepareV86Artifact = async (sourceProjectId) => {
		const file = editor.demoZip;
		const request = {
			source_project_id: sourceProjectId ?? null,
			system_version_id: Number(editingData.v86SystemVersionId),
			expected_artifact_revision: sourceProjectId ? editingData.v86ArtifactRevision : 0,
			manifest: editingData.v86Manifest,
			file_name: file?.name ?? null,
			size_bytes: file?.size ?? null
		};
		const start = await fetch('/api/v86/games/upload', {
			method: 'POST',
			headers: { Authorization: auth(), 'Content-Type': 'application/json' },
			body: JSON.stringify(request)
		});
		if (!start.ok) throw new Error(await start.text());
		const session = await start.json();
		try {
			if (session.upload_required) {
				editor.status = 'Uploading v86 game…';
				for (
					let index = session.next_chunk_index;
					index * session.chunk_size_bytes < file.size;
					index++
				) {
					const startByte = index * session.chunk_size_bytes;
					const chunk = file.slice(
						startByte,
						Math.min(startByte + session.chunk_size_bytes, file.size)
					);
					const result = await fetch(`/api/v86/games/upload/${session.upload_id}/chunk/${index}`, {
						method: 'PUT',
						headers: { Authorization: auth(), 'Content-Type': 'application/octet-stream' },
						body: chunk
					});
					if (!result.ok) throw new Error(await result.text());
					editor.status = `Uploading v86 game… ${Math.round(((startByte + chunk.size) / file.size) * 100)}%`;
				}
			}
			editor.status = 'Building v86 disc…';
			const complete = await fetch(`/api/v86/games/upload/${session.upload_id}/complete`, {
				method: 'POST',
				headers: { Authorization: auth() }
			});
			if (!complete.ok) throw new Error(await complete.text());
			const uploadId = session.upload_id;
			await new Promise((resolve, reject) => {
				const interval = setInterval(async () => {
					try {
						const res = await fetch(`/api/v86/games/upload/${uploadId}`, {
							headers: { Authorization: auth() }
						});
						if (!res.ok) { clearInterval(interval); reject(new Error(await res.text())); return; }
						const data = await res.json();
						if (data.chunk_progress?.message) editor.status = data.chunk_progress.message;
						if (data.status === 'ready') { clearInterval(interval); resolve(); }
						if (data.status === 'failed') {
							clearInterval(interval);
							await fetch(`/api/v86/games/upload/${uploadId}`, { method: 'DELETE', headers: { Authorization: auth() } }).catch(() => {});
							reject(new Error(data.error_message ?? 'Game build failed.'));
						}
					} catch (e) { clearInterval(interval); reject(e); }
				}, 800);
			});
			return uploadId;
		} catch (error) {
			await fetch(`/api/v86/games/upload/${session.upload_id}`, {
				method: 'DELETE',
				headers: { Authorization: auth() }
			}).catch(() => {});
			throw error;
		}
	};

	const uploadJsDosBundle = async (projectId, file) => {
		const start = await fetch(`/api/projects/id/${projectId}/jsdos/upload`, {
			method: 'POST',
			headers: { Authorization: auth(), 'Content-Type': 'application/json' },
			body: JSON.stringify({ file_name: file.name, size_bytes: file.size })
		});
		if (!start.ok) throw new Error(await start.text());
		const session = await start.json();
		editor.status = 'Uploading game…';
		for (
			let index = session.next_chunk_index;
			index * session.chunk_size_bytes < file.size;
			index++
		) {
			const startByte = index * session.chunk_size_bytes;
			const chunk = file.slice(
				startByte,
				Math.min(startByte + session.chunk_size_bytes, file.size)
			);
			const chunkRes = await fetch(
				`/api/projects/id/${projectId}/jsdos/upload/${session.upload_id}/chunk/${index}`,
				{
					method: 'PUT',
					headers: { Authorization: auth(), 'Content-Type': 'application/octet-stream' },
					body: chunk
				}
			);
			if (!chunkRes.ok) throw new Error(await chunkRes.text());
			const progress = Math.round(((startByte + chunk.size) / file.size) * 100);
			editor.status = `Uploading game… ${progress}%`;
		}
		const complete = await fetch(
			`/api/projects/id/${projectId}/jsdos/upload/${session.upload_id}/complete`,
			{ method: 'POST', headers: { Authorization: auth() } }
		);
		if (!complete.ok) throw new Error(await complete.text());
	};
</script>

{#snippet extraFields()}
	<div class="flex flex-col">
		<label for="demo-type">Demo type:</label>
		<select
			id="demo-type"
			class="px-1 min-w-0 bg-white rounded-sm"
			bind:value={editingData.demoType}
			disabled={!isOwner}
		>
			{#each demoTypes as type}
				<option value={type.value} disabled={type.disabled}>
					{type.label}{type.disabled ? ' (soon)' : ''}
				</option>
			{/each}
		</select>
	</div>
	{#if editingData.demoType !== 'none'}
		<div class="grid grid-cols-2 gap-2">
			<div class="flex flex-col">
				<label for="demo-width">Demo width:</label>
				<input
					id="demo-width"
					class="px-1 min-w-0 bg-white rounded-sm"
					bind:value={editingData.demoWidth}
					readonly={!isOwner}
				/>
			</div>
			<div class="flex flex-col">
				<label for="demo-height">Demo height:</label>
				<input
					id="demo-height"
					class="px-1 min-w-0 bg-white rounded-sm"
					bind:value={editingData.demoHeight}
					readonly={!isOwner}
				/>
			</div>
		</div>
	{/if}
	{#if editingData.demoType !== 'none' && editingData.demoType !== 'html5' && editingData.demoType !== 'webgl' && editingData.demoType !== 'jsdos' && editingData.demoType !== 'v86'}
		<div class="flex flex-col">
			<label for="demo-url">
				{#if editingData.demoType === 'embed'}
					Demo URL (required):
				{:else if editingData.demoType === 'download'}
					Download URL (required):
				{:else if editingData.demoType === 'video'}
					Video URL (required):
				{:else}
					URL:
				{/if}
			</label>
			<input
				id="demo-url"
				class="px-1 min-w-0 bg-white rounded-sm"
				bind:value={editingData.demoUrl}
				placeholder={editingData.demoType === 'video'
					? 'https://example.com/video.mp4'
					: editingData.demoType === 'download'
						? 'https://example.com/download-link'
						: 'https://example.github.io/my-demo/'}
				readonly={!isOwner}
			/>
		</div>
	{/if}
	{#if editingData.demoType === 'v86'}
		<div class="space-y-2 rounded-lg bg-white/70 p-2">
			<div class="flex flex-col">
				<label for="v86-system">System:</label>
				<select
					id="v86-system"
					class="px-1 min-w-0 bg-white rounded-sm"
					bind:value={editingData.v86SystemVersionId}
					disabled={!isOwner}
				>
					<option value="">Select a system</option>
					{#each v86Systems as system}
						{#each system.versions as version}
							<option value={version.id}>
								{system.name} v{version.version_number}{system.is_default ? ' (default)' : ''}
							</option>
						{/each}
					{/each}
				</select>
			</div>
			<div class="flex flex-col">
				<label for="v86-manifest">Manifest:</label>
				<textarea
					id="v86-manifest"
					rows="6"
					class="px-1 min-w-0 bg-white rounded-sm font-mono"
					bind:value={editingData.v86Manifest}
					readonly={!isOwner}></textarea>
				<p class="text-xs text-dark/60">
					v86 manifest keys: exe (required), plus optional args, delay_ms and save_paths. Paths are
					relative to the game drive root.
				</p>
			</div>
		</div>
	{/if}
	{#if editingData.demoType === 'html5' || editingData.demoType === 'webgl' || editingData.demoType === 'jsdos' || editingData.demoType === 'v86'}
		<div
			class="p-2 rounded-lg bg-white/70 border-2 border-dashed border-dark/30"
			ondrop={(e) => {
				e.preventDefault();
				setDemoZip(e.dataTransfer.files[0]);
			}}
			ondragover={preventDefault}
			role="none"
		>
			<label class="block font-semibold" for="demo-zip">
				{editingData.demoType === 'jsdos'
					? 'js-dos bundle:'
					: editingData.demoType === 'v86'
						? 'v86 game ZIP:'
						: editingData.demoType === 'html5'
							? 'HTML5 zip:'
							: 'WebGL zip:'}
			</label>
			<input
				id="demo-zip"
				type="file"
				accept={editingData.demoType === 'jsdos' ? '.jsdos' : '.zip,application/zip'}
				disabled={!isOwner}
				onchange={(e) => setDemoZip(e.currentTarget.files?.[0])}
			/>
			{#if editor.demoZipName}
				<p class="text-sm text-accent-green">{editor.demoZipName}</p>
			{:else if mode === 'edit'}
				<p class="text-sm text-dark/50">Leave empty to keep current demo.</p>
			{/if}
			{#if editor.demoZipError}
				<p class="text-sm text-accent-red">{editor.demoZipError}</p>
			{/if}
		</div>
	{/if}
	<div class="space-y-2">
		<div class="flex items-center justify-between">
			<span>External links:</span>
			<button
				class="px-2 rounded-sm bg-white"
				disabled={!isOwner}
				onclick={() => (editingData.links = [...editingData.links, { label: '', url: '' }])}
			>
				+
			</button>
		</div>
		{#each editingData.links as link, index}
			<div class="grid grid-cols-[1fr_1fr_auto] gap-1">
				<input
					class="px-1 min-w-0 bg-white rounded-sm"
					placeholder="Label"
					bind:value={link.label}
					readonly={!isOwner}
				/>
				<input
					class="px-1 min-w-0 bg-white rounded-sm"
					placeholder="URL"
					bind:value={link.url}
					readonly={!isOwner}
				/>
				<button
					class="px-2 rounded-sm bg-white text-accent-red"
					disabled={!isOwner}
					onclick={() => (editingData.links = editingData.links.filter((_, i) => i !== index))}
				>
					x
				</button>
			</div>
		{/each}
	</div>
{/snippet}

<PostEditorShell
	{mode}
	{isOwner}
	kind="project"
	routePrefix="/projects"
	dashboardPrefix="/dashboard/projects/id"
	coverApiPath={`/api/projects/id/${editingData.id}/cover`}
	titleLabel="Name"
	excerptRows={4}
	{editingData}
	{editor}
	{renderedText}
	{forDraft}
	{mediaSyntax}
	{mediaDictionary}
	{searchMedia}
	onToggleVersion={toggleVersion}
	onSubmit={newProject}
	onChange={updateProject}
	onPublish={publishProject}
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
