<script>
	import { goto } from '$app/navigation';
	import { auth, user } from '$lib/client/user';
	import { arraysEqualIgnoreOrder, nowToDate, preventDefault } from '$lib/common';
	import { useDebounce } from '$lib/effects/debounce';
	import { fly } from 'svelte/transition';
	import PostCard from '../home/PostCard.svelte';
	import PostSection from '../post/PostSection.svelte';
	import ContentDebounceEdtior from '../post/ContentDebounceEdtior.svelte';
	import MediaDictionaryController from '../post/MediaDictionaryController.svelte';
	import PBody from '../PBody.svelte';

	const mediaSyntax = /\@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]/g;
	const lottieAppSyntax = /:::app\s+lottie\s+([^\s]+)\s*/g;
	const allowedTypes = ['image/jpeg', 'image/png', 'image/gif', 'image/webp'];
	const maxFileSize = 5 * 1024 * 1024;
	const demoTypes = [
		{ value: 'html5', label: 'HTML5', disabled: false },
		{ value: 'embed', label: 'Embed', disabled: true },
		{ value: 'webgl', label: 'WebGL', disabled: true },
		{ value: 'download', label: 'Download', disabled: true },
		{ value: 'video', label: 'Video', disabled: true }
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
		links: [{ label: 'GitHub', url: '' }],
		author: {
			username: $user?.username,
			displayName: $user?.displayName,
			avatarUrl: $user?.avatarUrl
		}
	});

	let editor = $state({
		coverToggled: false,
		toggled: false,
		view: 'private',
		status: '',
		isCritical: false,
		coverFile: undefined,
		newCover: undefined,
		coverError: '',
		isUploadingCover: false,
		isUploadedCover: false,
		isPublishing: false,
		demoZip: undefined,
		demoZipName: '',
		demoZipError: ''
	});

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

	const newProject = async () => {
		if (!editor.demoZip) {
			editor.isCritical = true;
			editor.status = 'Demo zip is required';
			return;
		}

		const tags = editingData.tags
			.trim()
			.split(' ')
			.filter((tag) => tag !== '');
		const offlineKeys = collectOfflineKeys([editingData.draft]);
		if (!validateOfflineKeys(offlineKeys)) return;

		const formData = new FormData();
		formData.append(
			'project_data',
			new Blob(
				[
					JSON.stringify({
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
					})
				],
				{ type: 'application/json' }
			)
		);
		formData.append('demo_zip', editor.demoZip, editor.demoZip.name);
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

{#if editor.coverToggled}
	<PBody>
		<div class="sticky top-0 flex justify-center items-center w-full h-screen pointer-events-auto">
			<div
				class="absolute cursor-not-allowed inset-0 z-10"
				onclick={() => {
					editor.coverToggled = false;
					editor.coverFile = undefined;
					editor.newCover = undefined;
					editor.isUploadingCover = false;
					editor.isUploadedCover = false;
					editor.coverError = '';
				}}
				role="none"
			></div>
			<div class="w-fit h-fit space-y-4 bg-white rounded-3xl p-4 text-xl z-11" role="none">
				<div
					class="flex justify-center items-center w-60 h-60 bg-background/60 outline-4 outline-dark outline-dashed rounded-xl overflow-hidden"
					ondrop={(e) => {
						e.preventDefault();
						const file = e.dataTransfer.files[0];
						if (!file) {
							editor.coverError = 'File not found!';
							return;
						}
						if (!allowedTypes.includes(file.type)) {
							editor.coverError = 'Only JPEG, PNG, GIF, or WEBP are allowed.';
							return;
						}
						if (file.size > maxFileSize) {
							editor.coverError = `File size exceeds 5MB (${file.size} bytes)`;
							return;
						}
						editor.coverFile = file;
						editor.newCover = URL.createObjectURL(file);
						editor.isUploadingCover = false;
						editor.isUploadedCover = false;
					}}
					ondragover={preventDefault}
					role="none"
				>
					{#if editor.newCover}
						<img class="full object-cover" src={editor.newCover} alt="temporary project cover" />
					{:else}
						<span class="w-40 text-center select-none text-dark">Upload your image here</span>
					{/if}
				</div>
				{#if editor.coverError}
					<span class="inline-block w-60 text-accent-red">*{editor.coverError}</span>
				{/if}
				<div class="duo-btn duo-green">
					<button
						disabled={!editor.newCover || editor.isUploadedCover || editor.isUploadingCover}
						onclick={async () => {
							const formData = new FormData();
							formData.append('file', editor.coverFile, editor.coverFile.name);
							editor.isUploadedCover = false;
							editor.isUploadingCover = true;
							const res = await fetch(`/api/projects/id/${editingData.id}/cover`, {
								method: 'PATCH',
								headers: { Authorization: auth() },
								body: formData
							});
							if (res.ok) {
								editor.isUploadedCover = true;
								editor.isUploadingCover = false;
								editor.coverError = '';
								editingData.coverUrl = editor.newCover;
							} else {
								editor.isUploadingCover = false;
								editor.coverError = await res.text();
							}
						}}
					>
						Apply
					</button>
				</div>
				{#if editor.isUploadedCover}
					<span class="inline-block w-60 text-accent-green">*New cover uploaded succesfully!</span>
				{/if}
			</div>
		</div>
	</PBody>
{/if}

<article class="relative flex flex-col gap-4 pb-4 *:drop-shadow-xl">
	<div class="flex w-full">
		<div class="p-2 rounded-xl bg-white mx-auto w-120">
			<PostCard
				id={editingData.id}
				dashboardMode={true}
				title={editingData.title === '' ? '<Empty>' : editingData.title}
				slug={editingData.slug}
				excerpt={editingData.excerpt === '' ? '<Empty>' : editingData.excerpt}
				author={{ name: $user.displayName, slug: $user.username }}
				tags={editingData.tags.split(' ').filter((tag) => tag !== '')}
				src={editingData.coverUrl}
				stats={{ views: '#', likes: '#', comments_count: '#' }}
				routePrefix="/projects"
				dashboardPrefix="/dashboard/projects/id"
				onclick={() => {
					if (mode !== 'edit' || !isOwner) return;
					editor.coverToggled = true;
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
			<div class="flex justify-between p-2 w-full">
				<div class="flex gap-2">
					<div
						class="w-25 duo-btn"
						class:duo-green={!editor.toggled}
						class:duo-red={editor.toggled}
					>
						<button onclick={() => (editor.toggled = !editor.toggled)}>
							{editor.toggled ? 'Collapse' : 'Expand'}
						</button>
					</div>
					{#if mode === 'edit' && isOwner}
						<div in:fly={{ duration: 200 }} class="duo-btn duo-blue">
							<button
								onclick={() => {
									if (editor.view === 'public') {
										forceContent(editingData.draft);
										editor.view = 'private';
										editor.toggled = true;
									} else {
										forceContent(editingData.content);
										editor.view = 'public';
									}
								}}
							>
								Ver. {editor.view === 'public' ? 'Published' : 'Draft'}
							</button>
						</div>
					{/if}
				</div>
				<div class="flex gap-2">
					{#if editor.toggled}
						<div
							class="my-auto"
							class:text-accent-green={!editor.isCritical}
							class:text-accent-red={editor.isCritical}
						>
							<span>{editor.status}</span>
						</div>
						{#if mode === 'create'}
							<div in:fly={{ duration: 200 }} class="duo-btn duo-green">
								<button onclick={newProject}>Submit</button>
							</div>
						{:else if !isOwner}
							<div class="my-auto text-dark/50 text-sm italic"><span>View only</span></div>
						{:else}
							<div in:fly={{ duration: 200 }} class="duo-btn duo-green">
								<button onclick={updateProject}>Change</button>
							</div>
							<div in:fly={{ duration: 200 }} class="duo-btn duo-green">
								<button onclick={publishProject} disabled={editor.isPublishing}>
									{editor.isPublishing ? 'Publishing...' : 'Publish'}
								</button>
							</div>
						{/if}
					{/if}
				</div>
			</div>
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
									readonly={!isOwner}
								></textarea>
							</div>
							<div class="flex flex-col">
								<label for="excerpt">Excerpt:</label>
								<textarea
									id="excerpt"
									class="p-1 outline-none bg-white rounded-sm resize-none custom-scrollbar"
									rows="4"
									bind:value={editingData.excerpt}
									readonly={!isOwner}
								></textarea>
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
							<div
								class="p-2 rounded-lg bg-white/70 border-2 border-dashed border-dark/30"
								ondrop={(e) => {
									e.preventDefault();
									setDemoZip(e.dataTransfer.files[0]);
								}}
								ondragover={preventDefault}
								role="none"
							>
								<label class="block font-semibold" for="demo-zip">HTML5 zip:</label>
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
					<ContentDebounceEdtior
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
