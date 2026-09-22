<script>
	import { audiobooks, probeAudioDuration } from '$lib/api/audiobooks.js';
	import AudiobookPlayer from '$lib/components/audio/AudiobookPlayer.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import { formatClock, formatDurationLabel } from '$lib/utils/duration.js';
	import { fly } from 'svelte/transition';
	import { untrack } from 'svelte';

	let { data } = $props();

	// Read once: the id identifies the resource for the lifetime of this page.
	const audiobookId = untrack(() => data.audiobookId);

	let audiobook = $state(null);
	let loading = $state(true);
	let error = $state('');
	let notice = $state('');

	// Metadata draft, re-seeded whenever the server copy is reloaded.
	let meta = $state({ title: '', slug: '', description: '', translator: '', tags: '' });
	let savingMeta = $state(false);

	/** Per-file upload progress, keyed by file name. */
	let uploads = $state([]);
	let uploading = $state(false);
	let coverBusy = $state(false);

	let showPreview = $state(false);
	let pendingDelete = $state(false);
	let deleting = $state(false);
	let typedConfirm = $state('');

	let trackDurations = $derived(audiobook?.tracks?.map((t) => t.id).join('-') ?? '');

	async function load() {
		loading = true;
		error = '';
		try {
			const result = await audiobooks.details(audiobookId);
			audiobook = result.audiobook;
			meta = {
				title: audiobook.title,
				slug: audiobook.slug,
				description: audiobook.description ?? '',
				translator: audiobook.translator ?? '',
				tags: (audiobook.tags ?? []).map((tag) => tag.name).join(', ')
			};
		} catch (e) {
			error = e?.message ?? 'Could not load this audiobook.';
		} finally {
			loading = false;
		}
	}

	load();

	function flash(message) {
		notice = message;
		setTimeout(() => {
			if (notice === message) notice = '';
		}, 4000);
	}

	async function saveMeta() {
		savingMeta = true;
		error = '';
		try {
			await audiobooks.update(audiobookId, {
				title: meta.title,
				slug: meta.slug,
				description: meta.description,
				// An empty field clears the translator rather than ignoring it.
				translator: meta.translator.trim() || null,
				tags: meta.tags
					.split(',')
					.map((tag) => tag.trim())
					.filter(Boolean)
			});
			await load();
			flash('Saved.');
		} catch (e) {
			error = e?.message ?? 'Could not save the changes.';
		} finally {
			savingMeta = false;
		}
	}

	async function uploadCover(event) {
		const file = event.currentTarget.files?.[0];
		if (!file) return;

		coverBusy = true;
		error = '';
		try {
			const form = new FormData();
			form.append('file', file, file.name);
			await audiobooks.changeCover(audiobookId, form);
			await load();
			flash('Cover updated.');
		} catch (e) {
			error = e?.message ?? 'Could not upload the cover.';
		} finally {
			coverBusy = false;
			event.currentTarget.value = '';
		}
	}

	/**
	 * Strip the extension so the file name becomes a usable chapter title.
	 * A leading chapter marker ("Ch.1", "Chapter 2", optional punctuation
	 * after the number) is dropped: the player already shows "Chapter N of M".
	 */
	function titleFromFilename(name) {
		return name
			.replace(/\.[^.]+$/, '')
			.replace(/^\s*(?:ch\.?|chapter)\s*\d+\s*[-–—:._]*\s*/i, '')
			.replace(/[_-]+/g, ' ')
			.trim();
	}

	async function uploadTracks(files) {
		const list = Array.from(files ?? []);
		if (list.length === 0) return;

		uploading = true;
		error = '';
		uploads = list.map((file) => ({ name: file.name, status: 'pending', message: '' }));

		for (const [index, file] of list.entries()) {
			uploads[index] = { ...uploads[index], status: 'probing' };
			// Probe in the browser: the server has no audio decoder, and the
			// element has to read the header to play the file anyway.
			const duration = await probeAudioDuration(file);

			uploads[index] = { ...uploads[index], status: 'uploading' };
			try {
				const form = new FormData();
				form.append('file', file, file.name);
				form.append('title', titleFromFilename(file.name));
				if (duration) form.append('duration_seconds', String(duration));

				await audiobooks.addTrack(audiobookId, form);
				uploads[index] = { ...uploads[index], status: 'done' };
			} catch (e) {
				uploads[index] = {
					...uploads[index],
					status: 'failed',
					message: e?.message ?? 'Upload failed.'
				};
			}
		}

		uploading = false;
		await load();

		const failed = uploads.filter((u) => u.status === 'failed').length;
		flash(failed === 0 ? `Added ${list.length} track(s).` : `${failed} track(s) failed.`);
	}

	async function moveTrack(index, delta) {
		const target = index + delta;
		if (!audiobook || target < 0 || target >= audiobook.tracks.length) return;

		const order = audiobook.tracks.map((track) => track.id);
		[order[index], order[target]] = [order[target], order[index]];

		try {
			await audiobooks.reorderTracks(audiobookId, order);
			await load();
		} catch (e) {
			error = e?.message ?? 'Could not reorder the tracks.';
		}
	}

	async function renameTrack(track, title) {
		const next = title.trim();
		if (!next || next === track.title) return;
		try {
			await audiobooks.updateTrack(audiobookId, track.id, { title: next });
			track.title = next;
			audiobook = audiobook;
		} catch (e) {
			error = e?.message ?? 'Could not rename the track.';
		}
	}

	async function removeTrack(track) {
		try {
			await audiobooks.removeTrack(audiobookId, track.id);
			await load();
			flash('Track removed.');
		} catch (e) {
			error = e?.message ?? 'Could not remove the track.';
		}
	}

	async function setStatus(status) {
		try {
			await audiobooks.changeStatus(audiobookId, status);
			await load();
			flash(status === 'published' ? 'Published.' : 'Moved back to draft.');
		} catch (e) {
			error = e?.message ?? 'Could not change the status.';
		}
	}

	async function confirmDelete() {
		deleting = true;
		try {
			await audiobooks.remove(audiobookId);
			window.location.href = '/dashboard/audiobooks';
		} catch (e) {
			error = e?.message ?? 'Could not delete the audiobook.';
			deleting = false;
			pendingDelete = false;
		}
	}
</script>

<svelte:head>
	<title>Manage audiobook - Dashboard | Huu Thang's Blog</title>
</svelte:head>

<section class="flex flex-col gap-4 pb-8">
	<div class="bg-white rounded-xl p-4 flex items-center justify-between gap-2 flex-wrap">
		<div class="flex flex-col gap-1">
			<a href="/dashboard/audiobooks" class="text-sm text-dark/50">← All audiobooks</a>
			<h1 class="text-2xl font-semibold">{audiobook?.title ?? 'Audiobook'}</h1>
			{#if audiobook}
				<div class="flex items-center gap-2 flex-wrap text-sm">
					<span
						class="text-xs px-2 py-0.5 rounded-full {audiobook.status === 'published'
							? 'bg-accent-green text-white'
							: 'bg-primary/20 text-dark'}"
					>
						{audiobook.status}
					</span>
					<span class="text-dark/50 font-mono">/{audiobook.slug}</span>
					<span class="text-dark/50">
						{audiobook.tracks.length} chapter{audiobook.tracks.length === 1 ? '' : 's'}
						{#if formatDurationLabel(audiobook.total_duration_seconds)}
							· {formatDurationLabel(audiobook.total_duration_seconds)}
						{/if}
					</span>
				</div>
			{/if}
		</div>

		<div class="flex items-center gap-2 flex-wrap">
			{#if audiobook?.status === 'published'}
				<a
					href="/audiobooks/{audiobook.slug}"
					class="rounded-full border border-dark/20 px-4 py-1.5 text-sm no-underline! text-dark hover:bg-dark/5"
				>
					View public page
				</a>
				<button
					class="rounded-full border border-dark/20 px-4 py-1.5 text-sm hover:bg-dark/5"
					onclick={() => setStatus('draft')}
				>
					Unpublish
				</button>
			{:else}
				<button
					class="rounded-full bg-accent-green text-white px-4 py-1.5 text-sm font-medium hover:bg-accent-green/90"
					onclick={() => setStatus('published')}
				>
					Publish
				</button>
			{/if}
			<button
				class="rounded-full border border-accent-red text-accent-red px-4 py-1.5 text-sm hover:bg-accent-red/10"
				onclick={() => {
					typedConfirm = '';
					pendingDelete = true;
				}}
			>
				Delete
			</button>
		</div>
	</div>

	{#if error}
		<p class="bg-white rounded-xl p-4 text-accent-red">{error}</p>
	{/if}
	{#if notice}
		<p class="bg-white rounded-xl p-4 text-accent-green-dark">{notice}</p>
	{/if}

	{#if loading}
		<div class="bg-white rounded-xl p-4">
			<p class="py-6 text-center text-dark/40">Loading…</p>
		</div>
	{:else if audiobook}
		<!-- Metadata -->
		<form
			class="bg-white rounded-xl p-4 flex flex-col gap-3"
			onsubmit={(event) => {
				event.preventDefault();
				saveMeta();
			}}
		>
			<h2 class="font-semibold">Details</h2>

			<div class="grid grid-cols-1 md:grid-cols-2 gap-3">
				<label class="flex flex-col gap-1">
					<span class="text-sm text-dark/60">Title</span>
					<input
						class="rounded-lg border-2 border-dark/10 focus:border-dark px-2 py-1 bg-white"
						type="text"
						bind:value={meta.title}
						required
					/>
				</label>
				<label class="flex flex-col gap-1">
					<span class="text-sm text-dark/60">Slug</span>
					<input
						class="rounded-lg border-2 border-dark/10 focus:border-dark px-2 py-1 bg-white font-mono text-sm"
						type="text"
						bind:value={meta.slug}
						required
					/>
				</label>
			</div>

			<label class="flex flex-col gap-1">
				<span class="text-sm text-dark/60">Description</span>
				<textarea
					class="rounded-lg border-2 border-dark/10 focus:border-dark px-2 py-1 bg-white resize-y custom-scrollbar"
					rows="4"
					bind:value={meta.description}></textarea>
			</label>

			<div class="grid grid-cols-1 md:grid-cols-2 gap-3">
				<label class="flex flex-col gap-1">
					<span class="text-sm text-dark/60">Translator</span>
					<input
						class="rounded-lg border-2 border-dark/10 focus:border-dark px-2 py-1 bg-white"
						type="text"
						bind:value={meta.translator}
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
						bind:value={meta.tags}
						placeholder="Science Fiction, Full Cast"
					/>
				</label>
			</div>

			<div class="flex items-center gap-4 flex-wrap">
				<button
					class="rounded-full bg-dark text-white px-5 py-2 text-sm font-medium hover:bg-dark/90 disabled:opacity-50"
					type="submit"
					disabled={savingMeta}
				>
					{savingMeta ? 'Saving…' : 'Save details'}
				</button>

				<label class="flex items-center gap-2 text-sm text-dark/60">
					Cover
					<input
						class="text-sm"
						type="file"
						accept="image/png,image/jpeg,image/webp,image/gif"
						disabled={coverBusy}
						onchange={uploadCover}
					/>
				</label>
				{#if coverBusy}
					<span class="text-sm text-dark/40">Uploading…</span>
				{/if}
			</div>
		</form>

		<!-- Tracks -->
		<div class="bg-white rounded-xl p-4 flex flex-col gap-3">
			<div class="flex items-center justify-between gap-2 flex-wrap">
				<h2 class="font-semibold">Chapters</h2>
				<label
					class="rounded-full bg-primary text-white px-4 py-1.5 text-sm font-medium cursor-pointer hover:bg-primary/90 {uploading
						? 'opacity-50 pointer-events-none'
						: ''}"
				>
					{uploading ? 'Uploading…' : 'Add audio tracks'}
					<input
						class="hidden"
						type="file"
						multiple
						accept="audio/mpeg,audio/mp3,audio/ogg,audio/wav,audio/x-wav,.mp3,.ogg,.wav"
						disabled={uploading}
						onchange={(event) => {
							uploadTracks(event.currentTarget.files);
							event.currentTarget.value = '';
						}}
					/>
				</label>
			</div>

			{#if uploads.length}
				<ul class="flex flex-col gap-1 text-sm">
					{#each uploads as item (item.name)}
						<li class="flex items-center gap-2">
							<span
								class="w-2 h-2 rounded-full shrink-0 {item.status === 'done'
									? 'bg-accent-green'
									: item.status === 'failed'
										? 'bg-accent-red'
										: 'bg-primary animate-pulse'}"
							></span>
							<span class="line-clamp-1 grow">{item.name}</span>
							<span class="text-dark/50 shrink-0">
								{#if item.status === 'probing'}reading length…
								{:else if item.status === 'uploading'}uploading…
								{:else if item.status === 'done'}added
								{:else if item.status === 'failed'}
									<span class="text-accent-red">{item.message}</span>
								{:else}queued{/if}
							</span>
						</li>
					{/each}
				</ul>
			{/if}

			{#if audiobook.tracks.length === 0}
				<p class="py-6 text-center text-dark/40">
					No chapters yet. Upload MP3, OGG, or WAV files to build the playlist.
				</p>
			{:else}
				<ol class="flex flex-col divide-y divide-background">
					{#each audiobook.tracks as track, index (track.id)}
						<li class="flex items-center gap-2 py-2" in:fly={{ y: -6, duration: 150 }}>
							<span class="w-6 text-right text-sm text-dark/40 shrink-0">{track.number}</span>

							<div class="flex flex-col shrink-0">
								<button
									class="text-dark/40 hover:text-dark disabled:opacity-20 leading-none"
									disabled={index === 0}
									onclick={() => moveTrack(index, -1)}
									aria-label="Move {track.title} up"
									title="Move up"
								>
									▲
								</button>
								<button
									class="text-dark/40 hover:text-dark disabled:opacity-20 leading-none"
									disabled={index === audiobook.tracks.length - 1}
									onclick={() => moveTrack(index, 1)}
									aria-label="Move {track.title} down"
									title="Move down"
								>
									▼
								</button>
							</div>

							<input
								class="grow min-w-0 rounded-lg border-2 border-transparent hover:border-dark/10 focus:border-dark px-2 py-1 text-sm bg-transparent"
								type="text"
								value={track.title}
								aria-label="Chapter title"
								onchange={(event) => renameTrack(track, event.currentTarget.value)}
							/>

							<span class="text-xs text-dark/50 tabular-nums shrink-0 w-14 text-right">
								{formatClock(track.duration_seconds)}
							</span>

							<button
								class="text-accent-red text-sm hover:underline shrink-0"
								onclick={() => removeTrack(track)}
							>
								Remove
							</button>
						</li>
					{/each}
				</ol>
			{/if}
		</div>

		<!-- Preview -->
		<div class="bg-white rounded-xl p-4 flex flex-col gap-3">
			<div class="flex items-center justify-between gap-2">
				<h2 class="font-semibold">Preview</h2>
				<button
					class="rounded-full border border-dark/20 px-4 py-1.5 text-sm hover:bg-dark/5"
					onclick={() => (showPreview = !showPreview)}
				>
					{showPreview ? 'Hide player' : 'Open player'}
				</button>
			</div>
			<p class="text-sm text-dark/50">
				Plays exactly what listeners get, including sequential auto-advance.
			</p>

			{#if showPreview}
				{#key trackDurations}
					<AudiobookPlayer
						tracks={audiobook.tracks}
						title={audiobook.title}
						translator={audiobook.translator ?? ''}
						coverUrl={audiobook.url}
						storageKey={`preview:${audiobook.slug}`}
					/>
				{/key}
			{/if}
		</div>
	{/if}
</section>

<ConfirmDialog
	open={pendingDelete}
	title="Delete this audiobook?"
	description="The audiobook and its chapter list will be removed. The uploaded audio files stay in the media library."
	confirmLabel="Delete audiobook"
	bind:typedValue={typedConfirm}
	busy={deleting}
	onconfirm={confirmDelete}
	oncancel={() => (pendingDelete = false)}
/>
