<script>
	import { onMount, untrack, flushSync } from 'svelte';
	import { AudiobookPlayer } from '$lib/players/AudiobookPlayer.svelte.js';
	import { audiobookSession } from '$lib/players/AudiobookSession.svelte.js';
	import { audiobooks } from '$lib/api/audiobooks.js';
	import { formatClock, percentOf } from '$lib/utils/duration.js';

	let {
		tracks = [],
		/**
		 * Windowed chapter source, for a public book whose chapters arrive a
		 * window at a time. Absent for the dashboard preview, which is handed
		 * the editor's whole playlist instead.
		 */
		chapters = null,
		/**
		 * How many chapters the book has. `tracks` holds only what has arrived,
		 * so the count has to come from the book itself.
		 */
		trackCount = null,
		title = 'Audiobook',
		author = '',
		translator = '',
		coverUrl = null,
		storageKey = 'default',
		slug = null,
		/**
		 * Numeric audiobook id. The public page passes it so genuine listening
		 * is counted server-side; the dashboard preview deliberately omits it,
		 * so previewing a book never counts as plays.
		 */
		audiobookId = null,
		/**
		 * Vietnamese-translated books present their player in Vietnamese too, so
		 * the whole listening experience matches the book's language.
		 */
		vietnamese = false,
		/**
		 * Public pages hand the book to the site-wide session, so playback (and the
		 * mini player) survives navigating away. The dashboard preview stays local
		 * to the page: an unpublished draft must not take over the listener.
		 */
		persistent = true
	} = $props();

	// The player UI follows the book's language; every user-facing string in
	// this component (and the mini player's, via the claimed book) comes from
	// here so the two languages cannot drift apart.
	const t = $derived(
		vietnamese
			? {
					noTracks: 'Cuốn sách nói này chưa có chương nào.',
					chapter: 'Chương',
					resumeFrom: (time) => `Tiếp tục từ ${time}?`,
					resume: 'Tiếp tục',
					startOver: 'Nghe lại từ đầu',
					blocked: 'Trình duyệt đã chặn tự động phát. Nhấn phát để bắt đầu nghe.',
					seek: 'Tua trong chương',
					previous: 'Chương trước',
					previousTitle: 'Chương trước (p)',
					play: 'Phát',
					pause: 'Tạm dừng',
					playTitle: 'Phát (Space)',
					pauseTitle: 'Tạm dừng (Space)',
					next: 'Chương sau',
					nextTitle: 'Chương sau (n)',
					mute: 'Tắt tiếng',
					unmute: 'Bật tiếng',
					volume: 'Âm lượng',
					sleepChapter: 'Hẹn giờ: hết chương',
					sleepRemaining: (clock) => `Hẹn giờ: ${clock}`,
					sleepTimer: 'Hẹn giờ tắt',
					sleepMinutes: (minutes) => `${minutes} phút`,
					sleepEndOfChapter: 'Hết chương',
					sleepCancel: 'Hủy hẹn giờ',
					restart: 'Nghe lại',
					playlist: 'Chương',
					chapterCount: (n) => `${n} chương`,
					loadingChapters: (from, to) => `Đang tải chương ${from}–${to}…`,
					plays: 'Lượt nghe',
					sortToHigh: 'Sắp xếp chương từ thấp lên cao',
					sortToLow: 'Sắp xếp chương từ cao xuống thấp',
					sortToHighTitle: 'Thấp lên cao',
					sortToLowTitle: 'Cao xuống thấp',
					searchLabel: 'Tìm chương',
					searchPlaceholder: 'Tìm theo tên hoặc số chương',
					searchAria: 'Tìm chương theo tên hoặc số',
					noMatch: 'Không có chương nào khớp tìm kiếm.',
					playing: 'Đang phát',
					paused: 'Tạm dừng',
					showCurrentChapter: 'Xem chương đang phát',
					trackError: {
						aborted: 'Phát bị hủy bỏ.',
						network: 'Lỗi mạng khi tải chương này.',
						decode: 'Không thể giải mã chương này.',
						unsupported: 'Định dạng âm thanh này không được hỗ trợ.',
						generic: 'Không thể phát chương này.'
					}
				}
			: {
					noTracks: 'This audiobook has no tracks yet.',
					chapter: 'Chapter',
					resumeFrom: (time) => `Resume from ${time}?`,
					resume: 'Resume',
					startOver: 'Start over',
					blocked: 'Playback was blocked by the browser. Press play to start listening.',
					seek: 'Seek within chapter',
					previous: 'Previous chapter',
					previousTitle: 'Previous chapter (p)',
					play: 'Play',
					pause: 'Pause',
					playTitle: 'Play (Space)',
					pauseTitle: 'Pause (Space)',
					next: 'Next chapter',
					nextTitle: 'Next chapter (n)',
					mute: 'Mute',
					unmute: 'Unmute',
					volume: 'Volume',
					sleepChapter: 'Sleep: chapter',
					sleepRemaining: (clock) => `Sleep: ${clock}`,
					sleepTimer: 'Sleep timer',
					sleepMinutes: (minutes) => `${minutes} minutes`,
					sleepEndOfChapter: 'End of chapter',
					sleepCancel: 'Cancel timer',
					restart: 'Restart',
					playlist: 'Chapters',
					chapterCount: (n) => `${n} chapter${n === 1 ? '' : 's'}`,
					loadingChapters: (from, to) => `Loading chapters ${from}–${to}…`,
					plays: 'Plays',
					sortToHigh: 'Sort chapters low to high',
					sortToLow: 'Sort chapters high to low',
					sortToHighTitle: 'Low to high',
					sortToLowTitle: 'High to low',
					searchLabel: 'Search chapters',
					searchPlaceholder: 'Search by title or chapter number',
					searchAria: 'Search chapters by title or number',
					noMatch: 'No chapters match your search.',
					playing: 'Playing',
					paused: 'Paused',
					showCurrentChapter: 'Show the chapter that is playing',
					trackError: {
						aborted: 'Playback aborted.',
						network: 'Network error while loading this track.',
						decode: 'This track could not be decoded.',
						unsupported: 'This audio format is not supported.',
						generic: 'This track could not be played.'
					}
				}
	);

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
		tracks,
		// The chapter source travels with the book, so playback keeps fetching
		// windows long after the page that started it has been torn down.
		chapters,
		// Carried on the book so the mini player, which renders the claimed book
		// on other pages, can match this language.
		vietnamese
	}));
	const player = untrack(() =>
		persistent
			? audiobookSession.engineFor(book)
			: new AudiobookPlayer(tracks, { storageKey, chapters })
	);
	// Read once: which source a book uses never changes after it is built, and a
	// listener returning to a book is handed the live engine — with the windows
	// it has already fetched — rather than this page's copy of them.
	const chapterSource = untrack(() => player.chapters ?? chapters ?? null);
	const totalChapters = $derived(chapterSource?.total || trackCount || tracks.length);
	// The play beacon rides on the engine's meta so it keeps firing while the
	// mini player continues playback on other pages, long after this page is
	// gone. Fire-and-forget: counting must never disturb listening. The server
	// answers with the chapter's new total, which is rendered from here — a
	// local map, because page data is not a reactive proxy and mutating it
	// would not re-render the row.
	let acceptedPlays = $state({});
	const onTrackPlayed = audiobookId
		? (trackId) => {
				audiobooks
					.recordTrackPlay(audiobookId, trackId)
					.then((result) => {
						// A `null` body is a 204: the report was shed as a burst
						// (or the book is not published), so the counter did not
						// move and the row must not pretend it did.
						if (result) acceptedPlays[trackId] = result.play_count;
					})
					.catch(() => {});
			}
		: null;
	untrack(() =>
		player.setMeta({
			title,
			author,
			translator,
			coverUrl,
			onTrackPlayed
		})
	);

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

	/**
	 * Which edge of the chapter list the playing chapter has scrolled off
	 * through, or `null` while it is on screen where it belongs.
	 *
	 * A listener browses a long book by scrolling away from what is playing, and
	 * a row that simply left the screen told them nothing about the audio still
	 * running. Keeping one eye on the playhead at the edge it left through gives
	 * that back without putting the reader's own scrolling under their thumb.
	 */
	let pinnedEdge = $state(null);

	/** The chapter row that plays while it is on screen. */
	const pinnedTrack = $derived(pinnedEdge === null ? null : current);

	/** Take the reader back to the chapter that is playing. */
	function returnToPlayhead() {
		playlistEl
			?.querySelector(`[data-track-index="${player.index}"]`)
			?.scrollIntoView({ block: 'center', behavior: 'smooth' });
	}

	/**
	 * Watch the playing chapter's row and record which edge it left through.
	 *
	 * The bar is a *separate* element rather than the row itself sticking in
	 * place, and that is not a stylistic choice — it is what makes this
	 * measurable. A sticky row reports its stuck position from both
	 * `getBoundingClientRect` and `offsetTop`, so a pin decided by measuring the
	 * row would see it already back in view, drop the pin, and oscillate on
	 * every scroll frame. Here the observed row never moves visually, so the
	 * measurement stays honest.
	 */
	function observeRow(node, active) {
		let observer = null;

		const stop = () => {
			if (!observer) return;
			observer.disconnect();
			observer = null;
			pinnedEdge = null;
		};

		const start = () => {
			if (observer) return;
			observer = new IntersectionObserver(
				(entries) => {
					const entry = entries[entries.length - 1];
					const root = entry?.rootBounds;
					const box = entry?.boundingClientRect;
					if (!root || !box) {
						pinnedEdge = null;
						return;
					}
					// Off the top and off the bottom are different answers: the bar
					// pins to whichever side the reader went past, which keeps it
					// between them and the chapters they are browsing.
					if (box.bottom <= root.top) pinnedEdge = 'top';
					else if (box.top >= root.bottom) pinnedEdge = 'bottom';
					else pinnedEdge = null;
				},
				// `node.closest('ol')` rather than `playlistEl`: the list is bound
				// after its children render, and a null root here would silently
				// measure against the viewport instead.
				{ root: node.closest('ol'), threshold: 0 }
			);
			observer.observe(node);
		};

		// Only the chapter that is playing is watched. Attaching an observer to
		// every row would have each of them overwrite the pin with an answer
		// about its own position, and a 400-chapter book would run 400 of them.
		if (active) start();

		return {
			update: (isActive) => (isActive ? start() : stop()),
			destroy: stop
		};
	}

	/**
	 * Fetch a window once the row standing in for it scrolls into view.
	 *
	 * The list shows one reachable row per unloaded stretch, so this is what
	 * turns "scroll down the chapter list" into "load the next window" — without
	 * ever painting a row per chapter of a long book.
	 */
	function windowRow(node, start) {
		let observer = null;

		const watch = (value) => {
			observer?.disconnect();
			observer = null;
			if (value == null) return;
			observer = new IntersectionObserver(
				(entries) => {
					if (entries.some((entry) => entry.isIntersecting)) chapterSource?.ensure(value);
				},
				{ root: node.closest('ol'), rootMargin: '160px 0px' }
			);
			observer.observe(node);
		};

		watch(start);
		return { update: watch, destroy: () => observer?.disconnect() };
	}

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

	// Keep the playing chapter in sight in a long playlist, without hijacking
	// the scroll from a reader who has deliberately wandered off it.
	$effect(() => {
		const index = player.index;
		const list = playlistEl;
		if (!list) return;
		// `pinnedEdge` is read inside untrack on purpose: it is written from the
		// observer above, and tracking a value this component writes would re-run
		// the effect on every scroll.
		untrack(() => {
			// Chasing the playhead is only welcome while the reader is already
			// following it. Once they have scrolled away, yanking the list back on
			// every chapter change is the exact thing the bar exists to avoid.
			if (pinnedEdge !== null) return;
			list
				.querySelector(`[data-track-index="${index}"]`)
				?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
		});
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

	/**
	 * Whether the reader is browsing the book as a whole — a search, or the
	 * reversed order. Both are deliberate "show me the list" acts, so they ask
	 * the source for every remaining window rather than filtering the chapters
	 * that happen to have arrived.
	 */
	const browsingWholeBook = $derived(
		Boolean(chapterSource) && (!sortAsc || chapterQuery.trim() !== '')
	);

	$effect(() => {
		if (browsingWholeBook) chapterSource?.ensureAll();
	});

	/** A rendered chapter: its slot in the book, and the chapter itself. */
	const chapterRow = (row) => ({
		kind: 'chapter',
		index: row.index,
		track: chapterSource.trackAt(row.index)
	});

	/**
	 * The rows the list renders: one per chapter, plus a skeleton row for a
	 * window in flight and a single reachable row for each stretch of chapters
	 * nobody has asked for yet. A chapter still on its way keeps its place, so
	 * the numbers never shift under the reader.
	 */
	const playlistRows = $derived.by(() => {
		if (!chapterSource) {
			return displayTracks.map((track) => ({
				kind: 'chapter',
				index: tracks.indexOf(track),
				track
			}));
		}

		// Ascending is the source's own order, so its rows come through as they
		// are — the waiting ones included. Those rows are the handles that fetch
		// the rest of the book, so dropping them would leave the list showing
		// whatever the page happened to load first, with no way to reach the
		// chapters behind it.
		if (!browsingWholeBook) {
			return chapterSource.rows.map((row) => (row.kind === 'chapter' ? chapterRow(row) : row));
		}

		// Whole-book mode reads the source too, not the playlist the page was
		// rendered with: a chapter that arrived in a later window is just as
		// much a chapter of this book, and its slot is the source's to name.
		// Descending reverses the waiting rows as well, because the chapters
		// they stand for are the ones the top of that list is waiting for.
		const arrived = chapterSource.rows.filter((row) => row.kind === 'chapter').map(chapterRow);
		const ordered = sortAsc ? arrived : arrived.reverse();

		// A search is answered from the chapters that are here — it does not wait
		// on the rest, so it gets no waiting rows and no false "no match" — and
		// still asks for the whole book, so browsing is never partial.
		if (chapterQuery.trim()) {
			const query = normalizeSearchText(chapterQuery);
			return ordered
				.filter((row) => row.track)
				.map((row, order) => ({ row, order, score: chapterSearchScore(row.track, query) }))
				.filter(({ score }) => score !== null)
				.sort((a, b) => a.score - b.score || a.order - b.order)
				.map(({ row }) => row);
		}

		const waiting = chapterSource.rows.filter((row) => row.kind !== 'chapter');
		return sortAsc ? [...ordered, ...waiting] : [...waiting.reverse(), ...ordered];
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

<section
	class="flex flex-col gap-4 rounded-xl bg-white p-4 text-dark"
	lang={vietnamese ? 'vi' : undefined}
>
	{#if !persistent}
		<audio bind:this={audioEl} preload="metadata" class="hidden"></audio>
	{/if}

	{#if totalChapters === 0}
		<p class="py-8 text-center text-base text-dark/50">{t.noTracks}</p>
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
				{t.chapter}
				{current?.number ?? 1} — {current?.title ?? ''}
			</p>
			{@render reel()}
		</div>

		{#if player.interrupted}
			<p class="text-sm md:text-base text-accent-red">
				{t.blocked}
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
					aria-label={t.seek}
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
					aria-label={t.previous}
					title={t.previousTitle}
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
					aria-label={player.playing ? t.pause : t.play}
					title={player.playing ? t.pauseTitle : t.playTitle}
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
					aria-label={t.next}
					title={t.nextTitle}
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
						aria-label={player.muted ? t.unmute : t.mute}
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
						aria-label={t.volume}
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
							{t.sleepChapter}
						{:else if player.sleepMode}
							{t.sleepRemaining(formatClock(player.sleepRemaining))}
						{:else}
							{t.sleepTimer}
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
									{t.sleepMinutes(minutes)}
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
								{t.sleepEndOfChapter}
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
									{t.sleepCancel}
								</button>
							</li>
						{/if}
					</ul>
				</details>
			</div>

			<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="white">
				<button onclick={() => player.clearSaved()}>{t.restart}</button>
			</div>
		</div>

		<!-- Resume offer: a status line under the player, not a banner over it. -->
		{#if player.resumeOffer}
			<div
				class="flex flex-wrap items-center gap-3 rounded-xl border border-dark/15 bg-background/25 p-3 text-sm md:text-base"
			>
				<span class="text-base grow">
					{t.resumeFrom(formatClock(player.resumeOffer.time))}
				</span>
				<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="primary">
					<button onclick={() => player.acceptResume()}>{t.resume}</button>
				</div>
				<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="dark">
					<button onclick={() => player.dismissResume()}>{t.startOver}</button>
				</div>
			</div>
		{/if}

		<!-- Chapter list: flush with the player edges and separated by a quiet rule. -->
		<div
			class="-mx-4 -mb-4 flex flex-col gap-2 rounded-b-xl border-t border-dark/10 bg-dark/5 px-4 pt-4 pb-4 text-dark"
		>
			<div class="flex items-center justify-between">
				<h2 class="text-lg font-semibold">{t.playlist}</h2>
				<div class="flex items-center gap-2">
					<span class="text-sm text-dark/55 sm:text-base">
						{t.chapterCount(totalChapters)}
					</span>
					<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="white">
						<button
							class="p-1.5!"
							onclick={toggleSort}
							aria-label={sortAsc ? t.sortToLow : t.sortToHigh}
							title={sortAsc ? t.sortToLowTitle : t.sortToHighTitle}
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
				<span class="sr-only">{t.searchLabel}</span>
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
					placeholder={t.searchPlaceholder}
					aria-label={t.searchAria}
					class="w-full rounded-lg border-2 border-dark/15 bg-white py-2 pr-3 pl-9 text-sm text-dark outline-none placeholder:text-dark/40 focus:border-primary"
				/>
			</label>
			{#if playlistRows.length === 0}
				<p class="py-6 text-center text-sm text-dark/55">{t.noMatch}</p>
			{:else}
				<!-- The wrapper is what the now-playing bar is positioned against, so the
				     bar can overlay an edge of the list without taking a row of its own
				     out of a fixed-height scroll area. -->
				<div class="relative">
					<ol
						bind:this={playlistEl}
						class="custom-scrollbar flex flex-col max-h-96 overflow-y-auto divide-y divide-dark/10"
					>
						{#each playlistRows as row (row.kind === 'chapter' ? `chapter-${row.index}` : `waiting-${row.start ?? 'rest'}`)}
							{#if row.kind === 'chapter' && row.track}
								{@const index = row.index}
								{@const track = row.track}
								{@const active = isCurrent(track)}
								<li data-track-index={index} data-track-id={track.id} use:observeRow={active}>
									<button
										class="flex w-full items-center gap-3 border-l-2 px-2 py-2 text-left transition-colors {active
											? 'border-primary bg-primary/10'
											: 'border-transparent hover:bg-dark/5'}"
										onclick={() => player.load(index, { play: true })}
										aria-current={active ? 'true' : undefined}
										aria-busy={player.pendingIndex === index ? 'true' : undefined}
									>
										<span
											class="grow min-w-0 flex flex-col {player.pendingIndex === index
												? 'animate-pulse motion-reduce:animate-none'
												: ''}"
										>
											<!-- Mobile: number on its own line so the title can wrap -->
											<span class="md:hidden text-sm text-dark/50">Ch.{track.number}</span>
											<span
												class="font-['Baloo_2',Roboto,sans-serif] text-base font-medium line-clamp-2 md:line-clamp-1 {active
													? 'text-dark'
													: 'text-dark/70'}"
											>
												<span class="hidden md:inline">{t.chapter} {track.number} -</span>
												{track.title}
											</span>
											{#if player.failures[track.id]}
												<span class="text-base text-accent-red">
													{t.trackError[player.failures[track.id]] ?? t.trackError.generic}
												</span>
											{/if}
										</span>

										<span
											class="flex items-center gap-2 shrink-0 text-base text-dark/55 tabular-nums"
										>
											<!-- Ahead of the stats, because a row whose playing state cannot be
											     read at a glance is just another row. -->
											{#if active}
												<svg
													class="w-4 h-4 shrink-0 fill-primary {player.playing
														? 'animate-reel motion-reduce:animate-none'
														: ''}"
													viewBox="0 0 24 24"
													role="img"
													aria-label={player.playing ? t.playing : t.paused}
												>
													{#if player.playing}
														<path d="M7 5h4v14H7zm6 0h4v14h-4z" />
													{:else}
														<path d="M8 5l11 7-11 7z" />
													{/if}
												</svg>
											{/if}
											<span class="flex items-center gap-1" title={t.plays}>
												<svg
													class="w-3.5 h-3.5 fill-dark/40"
													viewBox="0 0 24 24"
													aria-hidden="true"
												>
													<path d="M8 5l11 7-11 7z" />
												</svg>
												{acceptedPlays[track.id] ?? track.play_count ?? 0}
											</span>
											{formatClock(track.duration_seconds)}
										</span>
									</button>
								</li>
							{:else}
								<!-- A window in flight, or a stretch of chapters nobody has
							     fetched: the row is the handle that fetches it, and it turns
							     into chapters as they arrive. -->
								<li
									class="flex w-full items-center gap-3 px-2 py-2"
									use:windowRow={row.start ?? row.index}
								>
									<span
										class="grow min-w-0 flex flex-col gap-1.5 animate-pulse motion-reduce:animate-none"
										aria-hidden="true"
									>
										<span class="h-4 w-3/5 rounded-full bg-dark/10"></span>
										<span class="h-3 w-1/3 rounded-full bg-dark/10 md:hidden"></span>
									</span>
									{#if row.kind === 'gap' && row.start != null}
										<span class="shrink-0 text-sm text-dark/45 tabular-nums">
											{t.loadingChapters(row.start + 1, row.start + row.count)}
										</span>
									{/if}
								</li>
							{/if}
						{/each}
					</ol>

					<!-- Now-playing bar: what the listener is hearing, held at the edge of the
					     list they scrolled the chapter off through. It is an overlay rather than a
					     reserved row so it cannot reflow the list as it appears. -->
					{#if pinnedEdge && pinnedTrack}
						<div data-pinned-bar={pinnedEdge} class="bg-primary-20 px-2">
							<div class="flex items-center gap-2 py-1">
								<button
									class="grow min-w-0 flex items-center gap-3 py-1 text-left"
									onclick={returnToPlayhead}
									title={t.showCurrentChapter}
								>
									<svg
										class="w-4 h-4 shrink-0 fill-primary {player.playing
											? 'animate-reel motion-reduce:animate-none'
											: ''}"
										viewBox="0 0 24 24"
										role="img"
										aria-label={player.playing ? t.playing : t.paused}
									>
										{#if player.playing}
											<path d="M7 5h4v14H7zm6 0h4v14h-4z" />
										{:else}
											<path d="M8 5l11 7-11 7z" />
										{/if}
									</svg>
									<span
										class="font-['Baloo_2',Roboto,sans-serif] grow min-w-0 text-base font-medium line-clamp-1"
									>
										<span class="hidden md:inline">{t.chapter} {pinnedTrack.number} -</span>
										<span class="md:hidden">Ch.{pinnedTrack.number}</span>
										{pinnedTrack.title}
									</span>
									<span class="shrink-0 text-sm tabular-nums text-dark/60">
										{formatClock(position)} / {formatClock(duration)}
									</span>
								</button>

								<div class="duo-btn w-fit shrink-0" data-duo-shape="round" data-duo-color="dark">
									<button
										class="p-1.5!"
										onclick={() => player.toggle()}
										aria-label={player.playing ? t.pause : t.play}
									>
										<svg class="w-5 h-5 fill-white" viewBox="0 0 24 24">
											{#if player.playing}
												<path d="M7 5h4v14H7zm6 0h4v14h-4z" />
											{:else}
												<path d="M8 5l11 7-11 7z" />
											{/if}
										</svg>
									</button>
								</div>
							</div>
							<!-- Chapter progress, so the bar reports how far in the chapter the
							     listener is and not only which chapter it is. -->
							<div class="h-1 w-full overflow-hidden rounded-full bg-dark/15">
								<div class="h-full bg-primary" style="width: {playedPercent}%"></div>
							</div>
						</div>
					{/if}
				</div>
			{/if}
		</div>

		<p class="sr-only" aria-live="polite">
			{player.playing ? t.playing : t.paused}
			{current?.title ?? ''}
		</p>
	{/if}
</section>
