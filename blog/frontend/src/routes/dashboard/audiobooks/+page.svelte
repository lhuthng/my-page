<script>
	import { audiobooks } from '$lib/api/audiobooks.js';
	import EmptyState from '$lib/components/dashboard/EmptyState.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import { formatDurationLabel } from '$lib/utils/duration.js';
	import { fly } from 'svelte/transition';

	let items = $state([]);
	let loading = $state(true);
	let loadError = $state('');

	let showForm = $state(false);
	let saving = $state(false);
	let formError = $state('');
	let slugState = $state('idle'); // idle | checking | free | taken

	let draft = $state({
		title: '',
		slug: '',
		description: '',
		translator: '',
		tags: '',
		file: null,
		slugTouched: false
	});

	let pendingDelete = $state(null);
	let deleting = $state(false);
	let typedConfirm = $state('');

	async function refresh() {
		loading = true;
		loadError = '';
		try {
			const result = await audiobooks.list({ limit: 100 });
			items = result.audiobooks ?? [];
		} catch (e) {
			loadError = e?.message ?? 'Could not load audiobooks.';
			items = [];
		} finally {
			loading = false;
		}
	}

	refresh();

	/** Mirror the backend's slug rules so the field validates before submit. */
	function slugify(value) {
		return value
			.trim()
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-+|-+$/g, '')
			.slice(0, 100);
	}

	let slugTimer;
	function onTitleInput(value) {
		draft.title = value;
		// Only auto-fill the slug while the author has not typed their own.
		if (!draft.slugTouched) draft.slug = slugify(value);
	}

	async function checkSlug() {
		if (!draft.slug) {
			slugState = 'idle';
			return;
		}
		slugState = 'checking';
		try {
			const { available } = await audiobooks.checkSlug(draft.slug);
			slugState = available ? 'free' : 'taken';
		} catch {
			slugState = 'idle';
		}
	}

	function onSlugInput(value) {
		draft.slug = slugify(value);
		draft.slugTouched = true;
		clearTimeout(slugTimer);
		slugTimer = setTimeout(checkSlug, 400);
	}

	function resetForm() {
		draft = {
			title: '',
			slug: '',
			description: '',
			translator: '',
			tags: '',
			file: null,
			slugTouched: false
		};
		slugState = 'idle';
		formError = '';
	}

	async function create() {
		formError = '';

		if (!draft.title.trim()) {
			formError = 'A title is required.';
			return;
		}
		if (!draft.slug) {
			formError = 'A slug is required.';
			return;
		}
		if (slugState === 'taken') {
			formError = 'That slug is already in use.';
			return;
		}

		saving = true;
		try {
			const form = new FormData();
			form.append('title', draft.title);
			form.append('slug', draft.slug);
			form.append('description', draft.description);
			if (draft.translator.trim()) form.append('translator', draft.translator);

			// The backend accepts one `tags` field per tag.
			for (const tag of draft.tags.split(',')) {
				const trimmed = tag.trim();
				if (trimmed) form.append('tags', trimmed);
			}

			if (draft.file) form.append('file', draft.file, draft.file.name);

			await audiobooks.create(form);
			resetForm();
			showForm = false;
			await refresh();
		} catch (e) {
			formError = e?.message ?? 'Could not create the audiobook.';
		} finally {
			saving = false;
		}
	}

	async function setStatus(audiobook, status) {
		try {
			await audiobooks.changeStatus(audiobook.id, status);
			audiobook.status = status;
			items = [...items];
		} catch (e) {
			loadError = e?.message ?? 'Could not change the status.';
		}
	}

	async function confirmDelete() {
		if (!pendingDelete) return;
		deleting = true;
		try {
			await audiobooks.remove(pendingDelete.id);
			items = items.filter((item) => item.id !== pendingDelete.id);
			pendingDelete = null;
			typedConfirm = '';
		} catch (e) {
			loadError = e?.message ?? 'Could not delete the audiobook.';
		} finally {
			deleting = false;
		}
	}

	function askDelete(audiobook) {
		pendingDelete = audiobook;
		typedConfirm = '';
	}
</script>

<svelte:head>
	<title>Audiobooks - Dashboard | Huu Thang's Blog</title>
</svelte:head>

<section class="flex flex-col gap-4 pb-8">
	<div class="bg-white rounded-xl p-4 flex items-center justify-between gap-2 flex-wrap">
		<h1 class="text-2xl font-semibold">
			Audiobooks
			<span class="text-dark/40 text-lg font-normal">({items.length})</span>
		</h1>
		<button
			class="rounded-full bg-dark text-white px-4 py-1.5 text-sm font-medium hover:bg-dark/90"
			onclick={() => (showForm = !showForm)}
		>
			{showForm ? 'Cancel' : 'New audiobook'}
		</button>
	</div>

	{#if showForm}
		<form
			class="bg-white rounded-xl p-4 flex flex-col gap-3"
			in:fly={{ y: -10, duration: 200 }}
			onsubmit={(event) => {
				event.preventDefault();
				create();
			}}
		>
			<h2 class="font-semibold">New audiobook</h2>

			<div class="grid grid-cols-1 md:grid-cols-2 gap-3">
				<label class="flex flex-col gap-1">
					<span class="text-sm text-dark/60">Title</span>
					<input
						class="rounded-lg border-2 border-dark/10 focus:border-dark px-2 py-1 bg-white"
						type="text"
						value={draft.title}
						oninput={(event) => onTitleInput(event.currentTarget.value)}
						required
					/>
				</label>

				<label class="flex flex-col gap-1">
					<span class="text-sm text-dark/60">
						Slug
						{#if slugState === 'checking'}
							<span class="text-dark/40">· checking…</span>
						{:else if slugState === 'free'}
							<span class="text-accent-green">· available</span>
						{:else if slugState === 'taken'}
							<span class="text-accent-red">· already taken</span>
						{/if}
					</span>
					<input
						class="rounded-lg border-2 px-2 py-1 bg-white font-mono text-sm
							{slugState === 'taken' ? 'border-accent-red' : 'border-dark/10 focus:border-dark'}"
						type="text"
						value={draft.slug}
						oninput={(event) => onSlugInput(event.currentTarget.value)}
						required
					/>
				</label>
			</div>

			<label class="flex flex-col gap-1">
				<span class="text-sm text-dark/60">Description</span>
				<textarea
					class="rounded-lg border-2 border-dark/10 focus:border-dark px-2 py-1 bg-white resize-y custom-scrollbar"
					rows="3"
					value={draft.description}
					oninput={(event) => (draft.description = event.currentTarget.value)}></textarea>
			</label>

			<div class="grid grid-cols-1 md:grid-cols-2 gap-3">
				<label class="flex flex-col gap-1">
					<span class="text-sm text-dark/60">Translator</span>
					<input
						class="rounded-lg border-2 border-dark/10 focus:border-dark px-2 py-1 bg-white"
						type="text"
						value={draft.translator}
						oninput={(event) => (draft.translator = event.currentTarget.value)}
						placeholder="Leave blank if untranslated"
					/>
				</label>

				<label class="flex flex-col gap-1">
					<span class="text-sm text-dark/60">
						Tags <span class="text-dark/40">· audiobook-only, comma separated</span>
					</span>
					<input
						class="rounded-lg border-2 border-dark/10 focus:border-dark px-2 py-1 bg-white"
						type="text"
						value={draft.tags}
						oninput={(event) => (draft.tags = event.currentTarget.value)}
						placeholder="Science Fiction, Full Cast"
					/>
				</label>
			</div>

			<label class="flex flex-col gap-1">
				<span class="text-sm text-dark/60">Cover image (optional)</span>
				<input
					class="text-sm"
					type="file"
					accept="image/png,image/jpeg,image/webp,image/gif"
					onchange={(event) => (draft.file = event.currentTarget.files?.[0] ?? null)}
				/>
			</label>

			{#if formError}
				<p class="text-sm text-accent-red">{formError}</p>
			{/if}

			<button
				class="self-end rounded-full bg-dark text-white px-5 py-2 text-sm font-medium hover:bg-dark/90 disabled:opacity-50"
				type="submit"
				disabled={saving}
			>
				{saving ? 'Creating…' : 'Create'}
			</button>
		</form>
	{/if}

	{#if loadError}
		<p class="bg-white rounded-xl p-4 text-accent-red">{loadError}</p>
	{/if}

	{#if loading}
		<div class="bg-white rounded-xl p-4">
			<p class="py-4 text-center text-dark/40">Loading audiobooks…</p>
		</div>
	{:else if items.length === 0}
		<div class="bg-white rounded-xl p-4">
			<EmptyState
				message="No audiobooks yet."
				hint="Create one, then upload its chapters as audio tracks."
				mascot
			/>
		</div>
	{:else}
		<ul class="flex flex-col gap-3">
			{#each items as audiobook (audiobook.id)}
				<li
					class="bg-white rounded-xl p-4 flex gap-4 not-lg:flex-col"
					in:fly={{ y: -10, duration: 250 }}
				>
					{#if audiobook.url}
						<img
							src={audiobook.url}
							alt=""
							class="w-20 h-20 rounded-lg object-cover shrink-0"
							loading="lazy"
						/>
					{:else}
						<div class="w-20 h-20 rounded-lg bg-primary/20 shrink-0"></div>
					{/if}

					<div class="flex flex-col gap-1 min-w-0 grow">
						<div class="flex items-center gap-2 flex-wrap">
							<h2 class="font-semibold text-lg">{audiobook.title}</h2>
							<span
								class="text-xs px-2 py-0.5 rounded-full {audiobook.status === 'published'
									? 'bg-accent-green text-white'
									: 'bg-primary/20 text-dark'}"
							>
								{audiobook.status}
							</span>
							{#if audiobook.owner_username}
								<span class="text-xs bg-dark/10 px-2 py-0.5 rounded-full text-dark/60">
									by {audiobook.owner_username}
								</span>
							{/if}
						</div>

						<span class="text-sm text-dark/50 font-mono">/{audiobook.slug}</span>

						{#if audiobook.translator}
							<p class="text-sm text-dark/60">By {audiobook.translator}</p>
						{/if}

						<div class="flex items-center gap-2 flex-wrap text-xs text-dark/60">
							<span>{audiobook.track_count} chapter{audiobook.track_count === 1 ? '' : 's'}</span>
							{#if formatDurationLabel(audiobook.total_duration_seconds)}
								<span aria-hidden="true">·</span>
								<span>{formatDurationLabel(audiobook.total_duration_seconds)}</span>
							{/if}
						</div>

						{#if audiobook.tags?.length}
							<ul class="flex flex-wrap gap-1">
								{#each audiobook.tags as tag}
									<li class="text-xs bg-primary/20 px-2 py-0.5 rounded-full">{tag}</li>
								{/each}
							</ul>
						{/if}
					</div>

					<div class="flex flex-col gap-2 shrink-0 not-lg:flex-row not-lg:flex-wrap">
						<a
							href="/dashboard/audiobooks/id/{audiobook.id}"
							class="rounded-full bg-primary text-white px-4 py-1.5 text-sm font-medium text-center no-underline! hover:bg-primary/90"
						>
							Manage
						</a>

						{#if audiobook.status === 'published'}
							<button
								class="rounded-full border border-dark/20 px-4 py-1.5 text-sm hover:bg-dark/5"
								onclick={() => setStatus(audiobook, 'draft')}
							>
								Unpublish
							</button>
						{:else}
							<button
								class="rounded-full border border-accent-green text-accent-green-dark px-4 py-1.5 text-sm hover:bg-accent-green/10"
								onclick={() => setStatus(audiobook, 'published')}
							>
								Publish
							</button>
						{/if}

						<button
							class="rounded-full border border-accent-red text-accent-red px-4 py-1.5 text-sm hover:bg-accent-red/10"
							onclick={() => askDelete(audiobook)}
						>
							Delete
						</button>
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</section>

<ConfirmDialog
	open={!!pendingDelete}
	title="Delete this audiobook?"
	description={`“${pendingDelete?.title ?? ''}” and its ${pendingDelete?.track_count ?? 0} chapter(s) will be removed. The uploaded audio files stay in the media library.`}
	confirmLabel="Delete audiobook"
	bind:typedValue={typedConfirm}
	busy={deleting}
	onconfirm={confirmDelete}
	oncancel={() => {
		pendingDelete = null;
		typedConfirm = '';
	}}
/>
