<script>
	import { goto } from '$app/navigation';
	import { auth, authState } from '$lib/auth/user.svelte.js';
	import { arraysEqualIgnoreOrder, nowToDate } from '$lib/utils';
	import { useDebounce } from '$lib/utils/debounce';
	import PostCard from '../home/PostCard.svelte';
	import PostSection from '../post/PostSection.svelte';
	import ContentDebounceEditor from '../post/ContentDebounceEditor.svelte';
	import MediaDictionaryController from '../post/MediaDictionaryController.svelte';
	import EditorToolbar from '../editor/EditorToolbar.svelte';
	import EditorCoverUploader from '../editor/EditorCoverUploader.svelte';
	import PBody from '../shell/PBody.svelte';

	const mediaSyntax = /\@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]/g;
	const lottieAppSyntax = /:::app\s+lottie\s+([^\s]+)\s*/g;
	const demoTypes = [
		{ value: 'html5', label: 'HTML5', disabled: false },
		{ value: 'embed', label: 'Embed', disabled: false },
		{ value: 'webgl', label: 'WebGL', disabled: false },
		{ value: 'download', label: 'Download', disabled: false },
		{ value: 'video', label: 'Video', disabled: false }
	];

	let { mode = 'create', data, isOwner = true } = $props();

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
		demoType: 'html5',
		demoWidth: '100%',
		demoHeight: '520px',
		demoUrl: '',
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
		demoZip: undefined,
		demoZipName: '',
		demoZipError: ''
	});

	let coverUploaderOpen = $state(false);

	let renderedText = $state('');
	let forDraft = $derived(mode === 'create' || (mode === 'edit' && editor.view === 'private'));

	if (mode === 'edit' && data !== undefined) {
		editingData.id = data.id;
		editingData.postId = data.postId;
		editingData.title = data.title;
		editingData.slug = data.slug;
		editingData.excerpt = data.excerpt;
		editingData.tags = data.tags.join(' ');
		editingData.content = data.content;
		editingData.draft = data.draft;
		editingData.coverUrl = data.coverUrl;
		editingData.demoType = data.demoType ?? 'html5';
		editingData.demoWidth = data.demoWidth ?? '100%';
		editingData.demoHeight = data.demoHeight ?? '520px';
		editingData.demoUrl = data.rawDemoUrl ?? '';
		editingData.links = data.links?.length ? data.links : [{ label: 'GitHub', url: '' }];
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
			case 'html5':
			case 'webgl':
				if (mode === 'create' && !zip) {
					return {
						valid: false,
						error: `Zip file is required for ${type.toUpperCase()} projects.`
					};
				}
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
		if (editingData.demoType !== 'html5' && editingData.demoType !== 'webgl') {
			projectPayload.demo_url = editingData.demoUrl;
		}
		formData.append(
			'project_data',
			new Blob([JSON.stringify(projectPayload)], { type: 'application/json' })
		);
		if (editor.demoZip) {
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
			goto(`/dashboard/projects/id/${id}`);
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
		if (editingData.demoWidth !== (data.demoWidth ?? '100%'))
			projectData.demo_width = editingData.demoWidth;
		if (editingData.demoHeight !== (data.demoHeight ?? '520px'))
			projectData.demo_height = editingData.demoHeight;
		if (
			editingData.demoType !== 'html5' &&
			editingData.demoType !== 'webgl' &&
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

		formData.append(
			'project_data',
			new Blob([JSON.stringify(projectData)], { type: 'application/json' })
		);
		if (editor.demoZip) formData.append('demo_zip', editor.demoZip, editor.demoZip.name);

		const res = await fetch('/api/projects/id/' + data.id, {
			method: 'PATCH',
			headers: { Authorization: auth() },
			body: formData
		});

		if (res.ok) {
			editor.isCritical = false;
			editor.status = 'OK!';
			clearNewMedia(offlineKeys);
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
		if (!file.name.toLowerCase().endsWith('.zip')) {
			editor.demoZipError = 'Only zip archives are allowed.';
			return;
		}
		editor.demoZip = file;
		editor.demoZipName = file.name;
	};
</script>

<EditorCoverUploader
	show={coverUploaderOpen}
	apiPath={`/api/projects/id/${editingData.id}/cover`}
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
				author={{ name: authState.user.displayName, slug: authState.user.username }}
				tags={editingData.tags.split(' ').filter((tag) => tag !== '')}
				src={editingData.coverUrl}
				stats={{ views: '#', likes: '#', comments_count: '#' }}
				routePrefix="/projects"
				dashboardPrefix="/dashboard/projects/id"
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
				onsubmit={newProject}
				onchange={updateProject}
				onpublish={publishProject}
			/>
			<div class="flex not-lg:flex-col gap-2 w-full h-full p-2 pt-1">
				<div class="flex grow gap-2">
					<div class="max-h-80 p-2 pr-1 w-1/3 bg-primary/40 rounded-lg">
						<div class="full space-y-2 pr-[3px] custom-scrollbar overflow-y-scroll">
							<div class="flex not-sm:flex-col">
								<label class="inline-block min-w-14" for="title">Name:</label>
								<input
									id="title"
									class="grow px-1 min-w-0 bg-white rounded-sm"
									bind:value={editingData.title}
									readonly={!isOwner}
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
									readonly={!isOwner}
								/>
							</div>
							<div class="flex flex-col">
								<label for="tags">Tags:</label>
								<textarea
									id="tags"
									class="p-1 outline-none bg-white rounded-sm resize-none custom-scrollbar"
									rows="2"
									bind:value={editingData.tags}
									readonly={!isOwner}></textarea>
							</div>
							<div class="flex flex-col">
								<label for="excerpt">Excerpt:</label>
								<textarea
									id="excerpt"
									class="p-1 outline-none bg-white rounded-sm resize-none custom-scrollbar"
									rows="4"
									bind:value={editingData.excerpt}
									readonly={!isOwner}></textarea>
							</div>
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
							{#if editingData.demoType !== 'html5' && editingData.demoType !== 'webgl'}
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
							{#if editingData.demoType === 'html5' || editingData.demoType === 'webgl'}
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
										{editingData.demoType === 'html5' ? 'HTML5' : 'WebGL'} zip:
									</label>
									<input
										id="demo-zip"
										type="file"
										accept=".zip,application/zip"
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
										onclick={() =>
											(editingData.links = [...editingData.links, { label: '', url: '' }])}
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
											onclick={() =>
												(editingData.links = editingData.links.filter((_, i) => i !== index))}
										>
											x
										</button>
									</div>
								{/each}
							</div>
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
