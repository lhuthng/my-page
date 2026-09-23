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

	// The player engine snapshots its `tracks` prop once, so the preview has to
	// remount when the playlist's identity changes — including a track's audio
	// being replaced, which keeps the id but swaps the media behind it.
	let playlistKey = $derived(
		audiobook?.tracks?.map((t) => `${t.id}:${t.short_name}`).join('-') ?? ''
	);

	function seedMeta() {
		if (!audiobook) return;
		meta = {
			title: audiobook.title,
			slug: audiobook.slug,
			description: audiobook.description ?? '',
			translator: audiobook.translator ?? '',
			tags: (audiobook.tags ?? []).map((tag) => tag.name).join(', ')
		};
	}

	/**
	 * Fetch the server copy. Track mutations pass `silent`, which keeps the
	 * editor mounted: flipping back to the loading placeholder collapses the
	 * page and throws the author to the top on every reorder or delete. A
	 * silent reload also leaves the unsaved metadata draft untouched.
	 */
	async function load({ silent = false } = {}) {
		if (!silent) {
			loading = true;
			error = '';
		}
		try {
			const result = await audiobooks.details(audiobookId);
			audiobook = result.audiobook;
			if (!silent) seedMeta();
		} catch (e) {
			error = e?.message ?? 'Could not load this audiobook.';
		} finally {
			if (!silent) loading = false;
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
			await load({ silent: true });
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
		await load({ silent: true });

		const failed = uploads.filter((u) => u.status === 'failed').length;
		flash(failed === 0 ? `Added ${list.length} track(s).` : `${failed} track(s) failed.`);
	}

	/** Renumber the playlist in place so the editor mirrors the server. */
	function resequence(tracks) {
		tracks.forEach((track, index) => (track.number = index + 1));
	}

	/**
	 * Persist a new track order. The list is already reordered locally, so a
	 * failure just restores the server's order rather than leaving the UI lying
	 * about what was saved — and it never blanks the page.
	 */
	async function persistOrder(order) {
		try {
			await audiobooks.reorderTracks(audiobookId, order);
		} catch (e) {
			error = e?.message ?? 'Could not reorder the tracks.';
			await load({ silent: true });
		}
	}

	/** Move the track at `from` so it ends up at index `to`, then persist. */
	async function moveTrack(from, to) {
		if (!audiobook || to < 0 || to >= audiobook.tracks.length || to === from) return;
		const tracks = [...audiobook.tracks];
		const [moved] = tracks.splice(from, 1);
		tracks.splice(to, 0, moved);
		resequence(tracks);
		audiobook.tracks = tracks;
		await persistOrder(tracks.map((track) => track.id));
	}

	async function renameTrack(track, title) {
		const next = title.trim();
		if (!next || next === track.title) return;
		try {
			await audiobooks.updateTrack(audiobookId, track.id, { title: next });
			track.title = next;
		} catch (e) {
			error = e?.message ?? 'Could not rename the track.';
		}
	}

	/**
	 * Remove a track optimistically: the row disappears immediately and the
	 * numbering closes up, so the list never collapses into a loading state and
	 * the author keeps their scroll position. A failed request puts it back.
	 */
	async function removeTrack(track) {
		const snapshot = [...audiobook.tracks];
		const tracks = snapshot.filter((item) => item.id !== track.id);
		resequence(tracks);
		audiobook.tracks = tracks;
		try {
			await audiobooks.removeTrack(audiobookId, track.id);
			flash('Track removed.');
		} catch (e) {
			resequence(snapshot);
			audiobook.tracks = snapshot;
			error = e?.message ?? 'Could not remove the track.';
		}
	}

	/** Swap a track's audio file, keeping its title and playlist position. */
	async function replaceTrack(track, file) {
		if (!file) return;
		replacingId = track.id;
		error = '';
		try {
			// The server has no audio decoder, so the new length is probed here.
			const duration = await probeAudioDuration(file);
			const form = new FormData();
			form.append('file', file, file.name);
			if (duration) form.append('duration_seconds', String(duration));
			await audiobooks.replaceTrack(audiobookId, track.id, form);
			await load({ silent: true });
			flash(`Replaced the audio for “${track.title}”.`);
		} catch (e) {
			error = e?.message ?? 'Could not replace the track audio.';
		} finally {
			replacingId = null;
		}
	}

	// ----- Drag to reorder ---------------------------------------------------
	// Reordering is driven by pointer events rather than native HTML5 drag and
	// drop: the same code works for mouse, pen and touch, and it does not fight
	// the text input that lives inside each row.
	/** Index of the row being dragged, or `null` when idle. */
	let dragIndex = $state(null);
	/** Index the dragged row will land on once dropped. */
	let dropIndex = $state(null);
	/** Row the drop line sits above; `null` means "after the last row". */
	let dropAnchorId = $state(null);
	/** Pointer travel since the drag started, used to follow the cursor. */
	let dragOffset = $state(0);
	/** True once the pointer has moved far enough to count as a drag. */
	let dragActive = $state(false);
	let dragRows = [];
	let dragStartY = 0;
	let dragRowTop = 0;
	let dragRowHeight = 0;
	/** Track id under a dragged file, so its row can highlight as a drop target. */
	let fileOverId = $state(null);
	/** True while audio files hover the chapters panel. */
	let fileDragActive = $state(false);
	/** Track id whose audio is currently being swapped. */
	let replacingId = $state(null);

	function onRowPointerDown(event, index) {
		if (event.button !== 0) return;
		// Controls inside the row keep their own behaviour.
		if (event.target.closest('input, button, label, a, select, textarea')) return;
		// Touch has no hover affordance and dragging the row body would fight
		// page scrolling, so on touch only the grip starts a drag.
		if (event.pointerType !== 'mouse' && !event.target.closest('[data-drag-handle]')) return;

		const list = event.currentTarget.closest('ol');
		if (!list) return;

		dragRows = [...list.querySelectorAll('li[data-track-row]')];
		const rect = event.currentTarget.getBoundingClientRect();
		dragIndex = index;
		dropIndex = index;
		dropAnchorId = null;
		dragOffset = 0;
		dragActive = false;
		dragStartY = event.clientY;
		// The row moves by transform, so its resting geometry is captured once.
		dragRowTop = rect.top;
		dragRowHeight = rect.height;
		event.preventDefault();
		event.currentTarget.setPointerCapture(event.pointerId);
	}

	function onRowPointerMove(event) {
		if (dragIndex === null) return;
		dragOffset = event.clientY - dragStartY;
		if (!dragActive && Math.abs(dragOffset) < 4) return;
		dragActive = true;

		// Count the rows whose midpoint sits above the dragged row's centre:
		// that count is exactly where it lands once removed and re-inserted.
		const centre = dragRowTop + dragOffset + dragRowHeight / 2;
		let slot = 0;
		for (let i = 0; i < dragRows.length; i++) {
			if (i === dragIndex) continue;
			const rect = dragRows[i].getBoundingClientRect();
			if (centre > rect.top + rect.height / 2) slot += 1;
		}
		dropIndex = slot;

		// The drop line goes above whichever row will follow the dragged one.
		const anchor = dragRows.filter((_, i) => i !== dragIndex)[slot] ?? null;
		dropAnchorId = anchor ? Number(anchor.dataset.trackId) : null;
	}

	function onRowPointerUp(event) {
		if (dragIndex === null) return;
		const from = dragIndex;
		const to = dropIndex;
		const moved = dragActive;
		if (event.currentTarget.hasPointerCapture?.(event.pointerId)) {
			event.currentTarget.releasePointerCapture(event.pointerId);
		}
		dragIndex = null;
		dropIndex = null;
		dropAnchorId = null;
		dragOffset = 0;
		dragActive = false;
		if (moved && to !== null && to !== from) moveTrack(from, to);
	}

	/** File drags are the only native drags left; `dataTransfer` marks them. */
	function isFileDrag(event) {
		const types = event.dataTransfer?.types;
		return types ? Array.from(types).includes('Files') : false;
	}

	function onRowFileDragOver(event, track) {
		if (!isFileDrag(event)) return;
		event.preventDefault();
		event.dataTransfer.dropEffect = 'copy';
		fileOverId = track.id;
	}

	function onRowFileDrop(event, track) {
		if (!isFileDrag(event)) return;
		// A file dropped on a row replaces its audio; stop it from bubbling to
		// the panel handler, which would append it as a new chapter instead.
		event.preventDefault();
		event.stopPropagation();
		fileOverId = null;
		fileDragActive = false;
		const file = event.dataTransfer.files?.[0];
		if (file) replaceTrack(track, file);
	}

	function onTracksDragEnter(event) {
		if (isFileDrag(event)) fileDragActive = true;
	}

	function onTracksDragLeave(event) {
		if (event.currentTarget.contains(event.relatedTarget)) return;
		fileDragActive = false;
		fileOverId = null;
	}

	function onTracksDrop(event) {
		if (!isFileDrag(event)) return;
		event.preventDefault();
		fileDragActive = false;
		fileOverId = null;
		uploadTracks(event.dataTransfer.files);
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

	<!-- Status toasts float above the page: a save/delete/error message must not
	     push the editor content down and shift the author's view. -->
	{#if error || notice}
		<div
			class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 w-[min(24rem,calc(100vw-2rem))]"
			aria-live="polite"
		>
			{#if error}
				<div
					class="flex items-start gap-2 rounded-xl bg-white border-l-4 border-accent-red shadow-xl px-4 py-3 text-accent-red"
				>
					<span class="grow min-w-0">{error}</span>
					<button
						class="shrink-0 text-lg leading-none"
						onclick={() => (error = '')}
						aria-label="Dismiss error"
					>
						×
					</button>
				</div>
			{/if}
			{#if notice}
				<p
					class="rounded-xl bg-white border-l-4 border-accent-green shadow-xl px-4 py-3 text-accent-green-dark"
				>
					{notice}
				</p>
			{/if}
		</div>
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

		<!-- Tracks: drop audio anywhere here to add chapters, or on a row to replace -->
		<div
			class="bg-white rounded-xl p-4 flex flex-col gap-3 {fileDragActive
				? 'outline-2 outline-dashed outline-primary outline-offset-2'
				: ''}"
			ondragenter={onTracksDragEnter}
			ondragover={(event) => {
				if (isFileDrag(event)) {
					event.preventDefault();
					event.dataTransfer.dropEffect = 'copy';
				}
			}}
			ondragleave={onTracksDragLeave}
			ondrop={onTracksDrop}
		>
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

			<p class="text-xs text-dark/40">
				Drag files in here to add chapters, or drop a file on a row to replace its audio. Drag any
				row — or its grip — up and down to reorder.
			</p>

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
					No chapters yet. Upload MP3, OGG, or WAV files to build the playlist — or drag them right
					here.
				</p>
			{:else}
				<ol class="flex flex-col">
					{#each audiobook.tracks as track, index (track.id)}
						{#if dragActive && dropAnchorId === track.id}
							<li class="h-1 my-1 rounded-full bg-primary" aria-hidden="true"></li>
						{/if}
						<li
							data-track-row
							data-track-id={track.id}
							class="border-b border-background last:border-b-0 {fileOverId === track.id
								? 'bg-primary/15 rounded-lg'
								: ''}"
							in:fly={{ y: -6, duration: 150 }}
							ondragover={(event) => onRowFileDragOver(event, track)}
							ondrop={(event) => onRowFileDrop(event, track)}
						>
							<div
								class="flex items-center gap-2 py-2 {dragActive && dragIndex === index
									? 'relative z-10 opacity-70 bg-white rounded-lg shadow-lg'
									: ''}"
								style={dragActive && dragIndex === index
									? `transform: translateY(${dragOffset}px)`
									: ''}
								onpointerdown={(event) => onRowPointerDown(event, index)}
								onpointermove={onRowPointerMove}
								onpointerup={onRowPointerUp}
								onpointercancel={onRowPointerUp}
							>
								<span
									data-drag-handle
									class="cursor-grab active:cursor-grabbing touch-none text-dark/40 hover:text-dark p-1 shrink-0"
									title="Drag to reorder"
									aria-hidden="true"
								>
									<svg class="w-4 h-4 fill-current" viewBox="0 0 24 24">
										<circle cx="9" cy="6" r="1.6" />
										<circle cx="15" cy="6" r="1.6" />
										<circle cx="9" cy="12" r="1.6" />
										<circle cx="15" cy="12" r="1.6" />
										<circle cx="9" cy="18" r="1.6" />
										<circle cx="15" cy="18" r="1.6" />
									</svg>
								</span>

								<span class="w-6 text-right text-sm text-dark/40 shrink-0">{track.number}</span>

								<div class="flex flex-col shrink-0">
									<button
										class="text-dark/40 hover:text-dark disabled:opacity-20 leading-none"
										disabled={index === 0}
										onclick={() => moveTrack(index, index - 1)}
										aria-label="Move {track.title} up"
										title="Move up"
									>
										▲
									</button>
									<button
										class="text-dark/40 hover:text-dark disabled:opacity-20 leading-none"
										disabled={index === audiobook.tracks.length - 1}
										onclick={() => moveTrack(index, index + 1)}
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

								<label
									class="text-sm text-dark/60 hover:text-dark shrink-0 cursor-pointer {replacingId !==
									null
										? 'opacity-50 pointer-events-none'
										: ''}"
								>
									{replacingId === track.id ? 'Replacing…' : 'Replace'}
									<input
										class="hidden"
										type="file"
										accept="audio/mpeg,audio/mp3,audio/ogg,audio/wav,audio/x-wav,.mp3,.ogg,.wav"
										disabled={replacingId !== null}
										onchange={(event) => {
											const file = event.currentTarget.files?.[0];
											event.currentTarget.value = '';
											replaceTrack(track, file);
										}}
									/>
								</label>

								<button
									class="text-accent-red text-sm hover:underline shrink-0"
									onclick={() => removeTrack(track)}
								>
									Remove
								</button>
							</div>
						</li>
					{/each}
					{#if dragActive && dragIndex !== null && dropAnchorId === null}
						<li class="h-1 my-1 rounded-full bg-primary" aria-hidden="true"></li>
					{/if}
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
				{#key playlistKey}
					<AudiobookPlayer
						tracks={audiobook.tracks}
						title={audiobook.title}
						translator={audiobook.translator ?? ''}
						coverUrl={audiobook.url}
						storageKey={`preview:${audiobook.slug}`}
						persistent={false}
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
