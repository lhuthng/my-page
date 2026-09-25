<script>
	import { onMount, untrack, flushSync } from 'svelte';
	import { AudiobookPlayer } from '$lib/players/AudiobookPlayer.svelte.js';
	import { audiobookSession } from '$lib/players/AudiobookSession.svelte.js';
	import { formatClock, percentOf } from '$lib/utils/duration.js';

	let {
		tracks = [],
		title = 'Audiobook',
		author = '',
		translator = '',
		coverUrl = null,
		storageKey = 'default',
		slug = null,
		/**
		 * Public pages hand the book to the site-wide session, so playback (and the
		 * mini player) survives navigating away. The dashboard preview stays local
		 * to the page: an unpublished draft must not take over the listener.
		 */
		persistent = true
	} = $props();

	// Built from a deliberate one-time read of the props: the engine owns the
	// playlist, so re-creating it whenever a prop reference changed would reset
	// playback. (`untrack` states that intent rather than leaving it implicit.)
	const book = untrack(() => ({
		id: storageKey,
		slug,
		title,
		author,
		translator,
		coverUrl,
		tracks
	}));
	const player = untrack(() =>
		persistent ? audiobookSession.engineFor(book) : new AudiobookPlayer(tracks, { storageKey })
	);
	untrack(() => player.setMeta({ title, author, translator, coverUrl }));

	let audioEl = $state(null);
	/** Value shown while the listener drags the scrubber. */
	let scrub = $state(null);
	let playlistEl = $state(null);

	const current = $derived(player.current);
	const position = $derived(scrub ?? player.time);
	const duration = $derived(player.displayDuration);
	const playedPercent = $derived(percentOf(position, duration));
	const bufferedPercent = $derived(percentOf(player.buffered, duration));
	const isCurrent = (track) => track.id === current?.id;

	$effect(() => {
		// A persistent engine is attached once by the session, to an element it
		// owns, so it survives this component being torn down on navigation.
		if (persistent || !audioEl) return;
		// attach() reads and writes engine state (restore() loads the saved
		// track), so it must not be tracked: tracking would re-run this effect
		// on every playlist/index change, and the re-attach's audio.load()
		// aborts any play() the user just triggered (AbortError) — the player
		// then shows "blocked by browser" instead of playing.
		untrack(() => player.attach(audioEl));
		return () => player.detach();
	});

	onMount(() => {
		if (!persistent) return;
		// Hide the mini player while this book's own player is on screen, and let
		// the session decide whether the engine outlives the page (it does while
		// it is the one playing).
		audiobookSession.onScreenId = book.id;
		return () => {
			if (audiobookSession.onScreenId === book.id) audiobookSession.onScreenId = null;
			audiobookSession.release(player);
		};
	});

	// Actually starting playback is what hands the book to the session, so merely
	// opening another audiobook never interrupts what is already playing.
	$effect(() => {
		if (persistent && player.playing) audiobookSession.claim(player, book);
	});

	// Keep the playing chapter visible in a long playlist.
	$effect(() => {
		const index = player.index;
		if (!playlistEl) return;
		const active = playlistEl.querySelector(`[data-track-index="${index}"]`);
		active?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
	});

	onMount(() => {
		const onKeydown = (event) => {
			if (event.metaKey || event.ctrlKey || event.altKey) return;

			const target = event.target;
			if (
				target instanceof HTMLElement &&
				(target.isContentEditable ||
					['INPUT', 'TEXTAREA', 'SELECT', 'BUTTON'].includes(target.tagName))
			) {
				// Let form controls keep their own keyboard behaviour: the
				// scrubber, volume slider, and speed picker all use arrows and
				// space themselves.
				return;
			}

			switch (event.key) {
				case ' ':
				case 'k':
					event.preventDefault();
					player.toggle();
					break;
				case 'ArrowRight':
					event.preventDefault();
					player.skip(event.shiftKey ? player.skipSeconds : 5);
					break;
				case 'ArrowLeft':
					event.preventDefault();
					player.skip(event.shiftKey ? -player.skipSeconds : -5);
					break;
				case 'ArrowUp':
					event.preventDefault();
					player.setVolume(player.volume + 0.05);
					break;
				case 'ArrowDown':
					event.preventDefault();
					player.setVolume(player.volume - 0.05);
					break;
				case 'n':
					player.next();
					break;
				case 'p':
					player.previous();
					break;
				case 'm':
					player.toggleMute();
					break;
				default:
					break;
			}
		};

		window.addEventListener('keydown', onKeydown);
		return () => window.removeEventListener('keydown', onKeydown);
	});

	function commitSeek(event) {
		const ratio = Number(event.currentTarget.value) / 1000;
		player.seekToRatio(ratio);
		scrub = null;
	}

	// The invisible range input maps pointer positions across
	// (width - native thumb width), so clicks near the edges land off from
	// the cursor. Pointer coordinates on the bar itself map exactly.
	function ratioFromPointer(event, el) {
		const rect = el.getBoundingClientRect();
		return Math.min(1, Math.max(0, (event.clientX - rect.left) / rect.width));
	}

	function previewScrub(event) {
		scrub = ratioFromPointer(event, event.currentTarget) * duration;
	}

	function commitPointerSeek(event) {
		player.seekToRatio(ratioFromPointer(event, event.currentTarget));
		scrub = null;
	}

	/** The volume bar maps the pointer the same way the scrubber does. */
	function setVolumeFromPointer(event) {
		player.setVolume(ratioFromPointer(event, event.currentTarget));
	}

	let sortAsc = $state(true);
	let chapterQuery = $state('');

	/** Small edit-distance helper for forgiving chapter-title searches. */
	function levenshtein(left, right) {
		if (left === right) return 0;
		if (!left.length) return right.length;
		if (!right.length) return left.length;

		const previous = Array.from({ length: right.length + 1 }, (_, index) => index);
		for (let leftIndex = 1; leftIndex <= left.length; leftIndex++) {
			const current = [leftIndex];
			for (let rightIndex = 1; rightIndex <= right.length; rightIndex++) {
				const substitution =
					previous[rightIndex - 1] + (left[leftIndex - 1] === right[rightIndex - 1] ? 0 : 1);
				current[rightIndex] = Math.min(
					current[rightIndex - 1] + 1,
					previous[rightIndex] + 1,
					substitution
				);
			}
			previous.splice(0, previous.length, ...current);
		}
		return previous[right.length];
	}

	function normalizeSearchText(value) {
		return String(value ?? '')
			.normalize('NFD')
			.toLowerCase()
			.replace(/\p{M}/gu, '')
			.replace(/\s+/g, ' ')
			.trim();
	}

	function chapterSearchScore(track, query) {
		const number = String(track.number);
		const title = normalizeSearchText(track.title);
		if (number === query) return 0;
		if (title.includes(query)) return 1;
		if (number.includes(query)) return 2;

		// Allow a typo for longer queries, while keeping one-character searches exact.
		if (query.length < 3) return null;
		const distance = levenshtein(query, title);
		return distance <= Math.max(2, Math.ceil(query.length * 0.4)) ? 3 + distance : null;
	}

	const displayTracks = $derived.by(() => {
		const ordered = sortAsc ? tracks : [...tracks].reverse();
		const query = normalizeSearchText(chapterQuery.trim());
		if (!query) return ordered;

		return ordered
			.map((track, order) => ({ track, order, score: chapterSearchScore(track, query) }))
			.filter((result) => result.score !== null)
			.sort((a, b) => a.score - b.score || a.order - b.order)
			.map((result) => result.track);
	});

	// FLIP: measure before the reorder, flush the DOM, then play from old → new.
	function toggleSort() {
		const first = playlistEl
			? new Map(
					[...playlistEl.children].map((li) => [li.dataset.trackId, li.getBoundingClientRect().top])
				)
			: null;
		sortAsc = !sortAsc;
		if (!first || matchMedia('(prefers-reduced-motion: reduce)').matches) return;
		flushSync();
		for (const li of playlistEl.children) {
			const dy = first.get(li.dataset.trackId) - li.getBoundingClientRect().top;
			if (dy)
				li.animate([{ transform: `translateY(${dy}px)` }, { transform: 'none' }], {
					duration: 300,
					easing: 'ease'
				});
		}
	}
</script>

<section class="flex flex-col gap-4 rounded-xl bg-white p-4 text-dark">
	{#if !persistent}
		<audio bind:this={audioEl} preload="metadata" class="hidden"></audio>
	{/if}

	{#if tracks.length === 0}
		<p class="py-8 text-center text-base text-dark/50">This audiobook has no tracks yet.</p>
	{:else}
		<!-- One cassette reel: a spoked hub that turns while a chapter plays. -->
		{#snippet reel()}
			<span
				class="relative w-4 h-4 sm:w-5 sm:h-5 shrink-0 overflow-hidden rounded-full border-2 border-dark/30 bg-dark/5 {player.playing
					? 'animate-reel motion-reduce:animate-none'
					: ''}"
				aria-hidden="true"
			>
				<svg class="absolute inset-0 w-full h-full fill-dark/40" viewBox="0 0 24 24">
					<rect x="10.75" width="2.5" height="8" rx="1.25" />
					<rect x="10.75" y="16" width="2.5" height="8" rx="1.25" />
					<rect width="8" y="10.75" height="2.5" rx="1.25" />
					<rect x="16" y="10.75" width="8" height="2.5" rx="1.25" />
					<circle cx="12" cy="12" r="3.5" />
				</svg>
			</span>
		{/snippet}

		<!-- Cassette label window: a flat paper label with two reels. -->
		<div
			class="flex items-center gap-2 rounded-xl border border-primary/15 bg-background/25 px-2 py-2 sm:gap-3 sm:px-3"
		>
			{@render reel()}
			<p
				class="grow min-w-0 text-center text-sm font-bold break-words line-clamp-2 sm:text-base md:text-xl"
			>
				Chapter {current?.number ?? 1} — {current?.title ?? ''}
			</p>
			{@render reel()}
		</div>

		<!-- Resume offer -->
		{#if player.resumeOffer}
			<div
				class="flex flex-wrap items-center gap-3 rounded-xl border border-dark/15 bg-background/25 p-3 text-sm md:text-base"
			>
				<span class="text-base grow">
					Resume from {formatClock(player.resumeOffer.time)}?
				</span>
				<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="primary">
					<button onclick={() => player.acceptResume()}>Resume</button>
				</div>
				<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="dark">
					<button onclick={() => player.dismissResume()}>Start over</button>
				</div>
			</div>
		{/if}

		{#if player.interrupted}
			<p class="text-sm md:text-base text-accent-red">
				Playback was blocked by the browser. Press play to start listening.
			</p>
		{/if}

		<!-- Progress -->
		<div class="flex items-center gap-2 sm:gap-3">
			<span class="text-sm md:text-base tabular-nums shrink-0">{formatClock(position)}</span>
			<div
				class="relative h-6 flex items-center grow cursor-pointer touch-none"
				onpointerdown={(event) => {
					event.currentTarget.setPointerCapture(event.pointerId);
					previewScrub(event);
				}}
				onpointermove={(event) => {
					if (event.buttons > 0) previewScrub(event);
				}}
				onpointerup={commitPointerSeek}
				onpointercancel={() => (scrub = null)}
			>
				<div class="absolute inset-x-0 h-3 rounded-full bg-dark/15 overflow-hidden">
					<div class="h-full bg-dark/25" style="width: {bufferedPercent}%"></div>
				</div>
				<div
					class="absolute left-0 h-3 rounded-full bg-primary pointer-events-none"
					style="width: {playedPercent}%"
				></div>
				<div
					class="absolute w-5 h-5 rounded-full bg-white border-[3px] border-primary shadow-md pointer-events-none"
					style="left: calc({playedPercent}% - 10px)"
				></div>
				<input
					type="range"
					min="0"
					max="1000"
					step="1"
					class="absolute inset-0 w-full opacity-0 pointer-events-none"
					aria-label="Seek within chapter"
					value={Math.round(playedPercent * 10)}
					oninput={(event) => (scrub = (Number(event.currentTarget.value) / 1000) * duration)}
					onchange={commitSeek}
				/>
			</div>
			<span class="text-sm md:text-base tabular-nums shrink-0">{formatClock(duration)}</span>
		</div>

		<!-- Transport: grouped on a soft panel so the controls read as a deck -->
		<div
			class="mx-auto flex w-fit items-center justify-center gap-4 rounded-xl bg-dark/5 px-6 py-3"
		>
			<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="dark">
				<button
					class="p-2!"
					disabled={!player.hasPrevious}
					onclick={() => player.previous()}
					aria-label="Previous chapter"
					title="Previous chapter (p)"
				>
					<svg class="w-6 h-6 fill-white" viewBox="0 0 24 24">
						<path d="M7 6h2v12H7zm3 6l9 6V6z" />
					</svg>
				</button>
			</div>

			<div
				class="duo-btn w-fit transition-[filter] duration-300 {player.playing
					? 'drop-shadow-[0_0_3px_var(--color-accent-red-dark)]'
					: ''}"
				data-duo-shape="round"
				data-duo-color={player.playing ? 'red' : 'green'}
			>
				<button
					class="p-4!"
					onclick={() => player.toggle()}
					aria-label={player.playing ? 'Pause' : 'Play'}
					title={player.playing ? 'Pause (Space)' : 'Play (Space)'}
				>
					{#if player.playing}
						<svg class="w-10 h-10 fill-white" viewBox="0 0 24 24">
							<path d="M7 5h4v14H7zm6 0h4v14h-4z" />
						</svg>
					{:else}
						<svg class="w-10 h-10 fill-white" viewBox="0 0 24 24">
							<path d="M8 5l11 7-11 7z" />
						</svg>
					{/if}
				</button>
			</div>

			<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="dark">
				<button
					class="p-2!"
					disabled={!player.hasNext}
					onclick={() => player.next()}
					aria-label="Next chapter"
					title="Next chapter (n)"
				>
					<svg class="w-6 h-6 fill-white" viewBox="0 0 24 24">
						<path d="M15 6h2v12h-2zM5 6l9 6-9 6z" />
					</svg>
				</button>
			</div>
		</div>

		<!-- Secondary controls -->
		<div class="flex items-center gap-2 sm:gap-3 flex-wrap justify-center text-sm md:text-base">
			<div class="flex items-center gap-2">
				<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="white">
					<button
						class="p-1.5!"
						onclick={() => player.toggleMute()}
						aria-label={player.muted ? 'Unmute' : 'Mute'}
					>
						<svg class="w-5 h-5 fill-dark" viewBox="0 0 24 24">
							{#if player.muted || player.volume === 0}
								<path
									d="M4 9v6h4l5 4V5L8 9H4zm12.5 3l2.5 2.5 1-1L17.5 11l2.5-2.5-1-1L16.5 10 14 7.5l-1 1L15.5 11 13 13.5l1 1z"
								/>
							{:else}
								<path d="M4 9v6h4l5 4V5L8 9H4zm12 3a4 4 0 0 0-2-3.46v6.92A4 4 0 0 0 16 12z" />
							{/if}
						</svg>
					</button>
				</div>
				<div
					class="relative h-5 w-24 flex items-center cursor-pointer touch-none"
					onpointerdown={(event) => {
						event.currentTarget.setPointerCapture(event.pointerId);
						setVolumeFromPointer(event);
					}}
					onpointermove={(event) => {
						if (event.buttons > 0) setVolumeFromPointer(event);
					}}
					onpointerup={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
				>
					<div class="absolute inset-x-0 h-3 rounded-full bg-dark/15"></div>
					<div
						class="absolute left-0 h-3 rounded-full bg-primary pointer-events-none"
						style="width: {player.volume * 100}%"
					></div>
					<div
						class="absolute w-4 h-4 rounded-full bg-white border-[3px] border-primary shadow pointer-events-none"
						style="left: calc({player.volume * 100}% - 8px)"
					></div>
					<input
						type="range"
						min="0"
						max="1"
						step="0.05"
						class="absolute inset-0 w-full opacity-0 pointer-events-none"
						aria-label="Volume"
						value={player.volume}
						oninput={(event) => player.setVolume(Number(event.currentTarget.value))}
					/>
				</div>
			</div>

			<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="white">
				<details class="relative">
					<summary class="list-none cursor-pointer">
						{player.rate}×
					</summary>
					<ul
						class="absolute bottom-full left-0 z-30 mb-2 overflow-hidden rounded-lg border border-dark/20 bg-white py-1 shadow-lg min-w-20"
					>
						{#each player.playbackRates as value}
							<li>
								<button
									class="w-full text-left text-base px-3 py-1 hover:bg-dark/10 {value ===
									player.rate
										? 'font-semibold'
										: ''}"
									onclick={(event) => {
										player.setRate(value);
										event.currentTarget.closest('details')?.removeAttribute('open');
									}}
								>
									{value}×
								</button>
							</li>
						{/each}
					</ul>
				</details>
			</div>

			<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="white">
				<details class="relative">
					<summary class="list-none cursor-pointer">
						{#if player.sleepMode === 'chapter'}
							Sleep: chapter
						{:else if player.sleepMode}
							Sleep: {formatClock(player.sleepRemaining)}
						{:else}
							Sleep timer
						{/if}
					</summary>
					<ul
						class="absolute bottom-full left-0 z-30 mb-2 overflow-hidden rounded-lg border border-dark/20 bg-white py-1 shadow-lg min-w-32"
					>
						{#each player.sleepPresets as minutes}
							<li>
								<button
									class="w-full text-left text-base px-3 py-1 hover:bg-dark/10"
									onclick={(event) => {
										player.startSleep(minutes);
										event.currentTarget.closest('details')?.removeAttribute('open');
									}}
								>
									{minutes} minutes
								</button>
							</li>
						{/each}
						<li>
							<button
								class="w-full text-left text-base px-3 py-1 hover:bg-dark/10"
								onclick={(event) => {
									player.startSleep('chapter');
									event.currentTarget.closest('details')?.removeAttribute('open');
								}}
							>
								End of chapter
							</button>
						</li>
						{#if player.sleepMode}
							<li>
								<button
									class="w-full text-left text-base px-3 py-1 text-accent-red hover:bg-dark/10"
									onclick={(event) => {
										player.cancelSleep();
										event.currentTarget.closest('details')?.removeAttribute('open');
									}}
								>
									Cancel timer
								</button>
							</li>
						{/if}
					</ul>
				</details>
			</div>

			<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="white">
				<button onclick={() => player.clearSaved()}>Restart</button>
			</div>
		</div>

		<!-- Chapter list: flush with the player edges and separated by a quiet rule. -->
		<div
			class="-mx-4 -mb-4 flex flex-col gap-2 rounded-b-xl border-t border-dark/10 bg-dark/5 px-4 pt-4 pb-4 text-dark"
		>
			<div class="flex items-center justify-between">
				<h2 class="text-lg font-semibold">Chapters</h2>
				<div class="flex items-center gap-2">
					<span class="text-sm text-dark/55 sm:text-base">
						{tracks.length} chapter{tracks.length === 1 ? '' : 's'}
					</span>
					<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="white">
						<button
							class="p-1.5!"
							onclick={toggleSort}
							aria-label={sortAsc ? 'Sort chapters high to low' : 'Sort chapters low to high'}
							title={sortAsc ? 'High to low' : 'Low to high'}
						>
							<svg
								class="w-5 h-5 fill-dark transition-transform {sortAsc ? '' : 'rotate-180'}"
								viewBox="0 0 24 24"
							>
								<path d="M3 18h6v-2H3v2zM3 6v2h18V6H3zm0 7h12v-2H3v2z" />
							</svg>
						</button>
					</div>
				</div>
			</div>

			<label class="relative block">
				<span class="sr-only">Search chapters</span>
				<svg
					class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 fill-dark/40"
					viewBox="0 0 24 24"
					aria-hidden="true"
				>
					<path
						d="M9.5 3a6.5 6.5 0 1 0 4.06 11.58l4.43 4.43 1.42-1.42-4.43-4.43A6.5 6.5 0 0 0 9.5 3Zm0 2a4.5 4.5 0 1 1 0 9 4.5 4.5 0 0 1 0-9Z"
					/>
				</svg>
				<input
					type="search"
					value={chapterQuery}
					oninput={(event) => (chapterQuery = event.currentTarget.value)}
					placeholder="Search by title or chapter number"
					aria-label="Search chapters by title or number"
					class="w-full rounded-lg border-2 border-dark/15 bg-white py-2 pr-3 pl-9 text-sm text-dark outline-none placeholder:text-dark/40 focus:border-primary"
				/>
			</label>

			{#if displayTracks.length === 0}
				<p class="py-6 text-center text-sm text-dark/55">No chapters match your search.</p>
			{:else}
				<ol
					bind:this={playlistEl}
					class="custom-scrollbar flex flex-col max-h-96 overflow-y-auto divide-y divide-dark/10"
				>
					{#each displayTracks as track (track.id)}
						{@const index = tracks.indexOf(track)}
						{@const active = isCurrent(track)}
						<li data-track-index={index} data-track-id={track.id}>
							<button
								class="flex w-full items-center gap-3 border-l-2 px-2 py-2 text-left transition-colors {active
									? 'border-primary bg-primary/10'
									: 'border-transparent hover:bg-dark/5'}"
								onclick={() => player.load(index, { play: true })}
								aria-current={active ? 'true' : undefined}
							>
								<span class="grow min-w-0 flex flex-col">
									<!-- Mobile: number on its own line so the title can wrap -->
									<span class="md:hidden text-sm text-dark/50">Ch.{track.number}</span>
									<span
										class="font-['Baloo_2',Roboto,sans-serif] text-base font-medium line-clamp-2 md:line-clamp-1 {active
											? 'text-dark'
											: 'text-dark/70'}"
									>
										<span class="hidden md:inline">Chapter {track.number} -</span>
										{track.title}
									</span>
									{#if player.failures[track.id]}
										<span class="text-base text-accent-red">{player.failures[track.id]}</span>
									{/if}
								</span>

								<span class="text-base text-dark/55 tabular-nums shrink-0">
									{formatClock(track.duration_seconds)}
								</span>
							</button>
						</li>
					{/each}
				</ol>
			{/if}
		</div>

		<p class="sr-only" aria-live="polite">
			{player.playing ? 'Playing' : 'Paused'}
			{current?.title ?? ''}
		</p>
	{/if}
</section>
