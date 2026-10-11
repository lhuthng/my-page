/**
 * Playback engine for an audiobook.
 *
 * Deliberately owns no markup: it drives a single `<audio>` element and exposes
 * reactive state, so the Svelte component only renders controls. Mirrors the
 * `players/*.svelte.js` convention used by the game players.
 *
 * Streaming, in either of the two ways a chapter can be stored:
 *
 * **Plain** — `src` is pointed at the chapter's media URL and the browser does
 * the work: ordinary HTTP requests with Range requests to buffer ahead and to
 * seek, so a multi-hour file is never held in memory.
 *
 * **Chunked** — a chapter stored as zstd chunks is decompressed here instead,
 * one chunk at a time (`audio-chunks.js`), and handed to the element as a Blob.
 * The bytes then come straight from the bucket on signed URLs rather than
 * through the backend, which is the point: a play costs the server nothing. The
 * price is that a Blob is its own timeline, so the decompressed bytes always
 * start at the chapter's first byte and a seek extends that prefix rather than
 * jumping into the middle of it.
 *
 * Which one a book uses is asked once, not per chapter — see `#probeChunked`.
 * Until the answer arrives a chapter streams plainly, so nothing ever waits on
 * the question, and a book that has no chunks at all costs one request.
 *
 * Only one track is ever loading.
 *
 * The chapter list itself may also arrive a window at a time (`chapters`),
 * which is why `load` can be handed a chapter that has not been fetched yet:
 * it asks for that window and re-enters once it lands, and keeps the few
 * chapters ahead of the playhead in hand so the boundary is never a stall.
 */

import { ChunkedAudio, offsetForTime, readChunkTable } from './audio-chunks.js';

const STORAGE_PREFIX = 'audiobook-player:';
/** Writing localStorage on every `timeupdate` would fire ~4x/second. */
const PERSIST_INTERVAL_MS = 4000;
const PLAYBACK_RATES = [0.75, 1, 1.25, 1.5, 1.75, 2];
const SLEEP_PRESETS_MINUTES = [15, 30, 45, 60];
/**
 * Real seconds of listening per recorded play. The counter measures the time a
 * listener actually spends listening — ten seconds is ten seconds whether the
 * book is playing at 0.75x or 2x — so a chapter gains a play for every one of
 * these blocks and keeps gaining them for as long as the listener keeps going.
 */
const PLAY_REPORT_INTERVAL_SECONDS = 10;

export class AudiobookPlayer {
	/** @type {Array<object>} ordered playlist */
	tracks = $state([]);
	index = $state(0);
	playing = $state(false);
	/** True while the browser has refused to start playback without a gesture. */
	interrupted = $state(false);
	time = $state(0);
	/** Duration reported by the element; falls back to stored metadata. */
	duration = $state(0);
	buffered = $state(0);
	rate = $state(1);
	volume = $state(1);
	muted = $state(false);
	skipSeconds = $state(15);
	/** trackId -> stable error key (`trackError` in the component localizes it). */
	failures = $state({});
	/** null | minutes (number) | 'chapter' */
	sleepMode = $state(null);
	/** Seconds left on a timed sleep; 0 when no timer is running. */
	sleepRemaining = $state(0);
	/** A saved position was found and is offered as a resume. */
	resumeOffer = $state(null);
	/**
	 * The chapter waiting on a window fetch, or null. The list renders that row
	 * as loading, so a chapter still on its way never looks like a broken one.
	 */
	pendingIndex = $state(null);

	#audio = null;
	/**
	 * Whether this book's chapters are stored as zstd chunks: `null` until asked,
	 * `true` for a chunked book, `false` for one that plays plainly. Three-valued
	 * on purpose — "not asked yet" and "asked, and the answer is no" lead to
	 * different behaviour, and only the first one may wait on a probe.
	 */
	#chunkedSupport = null;
	/** The probe in flight, so a load can wait for it instead of guessing. */
	#probe = null;
	/**
	 * Whether that probe has answered. Tracked apart from `#probe` because a
	 * probe that answers "there was nothing to ask about" leaves the question
	 * open while its promise is still the most recent one: without this a load
	 * would wait on an already-settled probe, re-enter itself, and wait again.
	 */
	#probeSettled = false;
	/** The chunked source of the chapter playing, or null when it is plain. */
	#chunks = null;
	/** The Blob URL `src` points at, kept so it can be revoked rather than leaked. */
	#chunkUrl = null;
	#storageKey = null;
	#lastPersist = 0;
	#sleepTicker = null;
	#sleepDeadline = 0;
	#pendingSeek = 0;
	#handlers = null;
	#detached = false;
	/** Real seconds of listening accumulated for the current chapter. */
	#playSeconds = 0;
	/** The element's currentTime at the previous timeupdate, for deltas. */
	#lastAudioTime = 0;

	constructor(tracks, { storageKey = 'default', skipSeconds = 15, chapters = null } = {}) {
		// A windowed book's playlist belongs to its source, which grows it one
		// window at a time; the array handed in here is only the first of those
		// windows, so taking it would freeze the playlist at whatever had arrived
		// by the time the player was built.
		this.chapters = chapters;
		this.tracks = chapters ? chapters.tracks : (tracks ?? []);
		this.#storageKey = STORAGE_PREFIX + storageKey;
		this.skipSeconds = skipSeconds;
	}

	get current() {
		return this.trackAt(this.index);
	}

	/** Chapters in the book, fetched or not. */
	get length() {
		return this.chapters ? this.chapters.total : this.tracks.length;
	}

	/** The chapter at `index`, or null while its window is still on its way. */
	trackAt(index) {
		return this.tracks[index] ?? null;
	}

	get hasNext() {
		return this.index < this.length - 1;
	}

	get hasPrevious() {
		return this.index > 0;
	}

	get totalDuration() {
		return this.tracks.reduce((sum, track) => sum + (track.duration_seconds ?? 0), 0);
	}

	/**
	 * Duration to display: the chapter's own length when it is chunked, else the
	 * live element value, else the stored metadata.
	 *
	 * A chunked chapter is played from a prefix of itself, so the element's
	 * duration is how much has been decompressed rather than how long the
	 * chapter is — a scrubber drawn from it would grow as the listener listened.
	 * The stored length is the truth in that case.
	 */
	get displayDuration() {
		if (this.#chunks && this.current?.duration_seconds) return this.current.duration_seconds;
		if (Number.isFinite(this.duration) && this.duration > 0) return this.duration;
		return this.current?.duration_seconds ?? 0;
	}

	get playbackRates() {
		return PLAYBACK_RATES;
	}

	get sleepPresets() {
		return SLEEP_PRESETS_MINUTES;
	}

	// -----------------------------------------------------------------------
	// Wiring
	// -----------------------------------------------------------------------

	/** Bind to the `<audio>` element and restore any saved session. */
	attach(audio) {
		if (!audio) return;
		this.#audio = audio;
		this.#detached = false;

		const handlers = {
			timeupdate: () => this.#onTimeUpdate(),
			loadedmetadata: () => this.#onLoadedMetadata(),
			durationchange: () => this.#onLoadedMetadata(),
			progress: () => this.#onProgress(),
			ended: () => this.#onEnded(),
			play: () => {
				this.playing = true;
				this.interrupted = false;
			},
			pause: () => {
				this.playing = false;
				this.#persist(true);
			},
			error: () => this.#onError(),
			ratechange: () => {
				this.rate = audio.playbackRate;
			},
			volumechange: () => {
				this.volume = audio.volume;
				this.muted = audio.muted;
			}
		};

		for (const [event, handler] of Object.entries(handlers)) {
			audio.addEventListener(event, handler);
		}
		this.#handlers = handlers;

		this.#restore();
		this.setupMediaSession();
		// Asked here rather than on the first chapter's load so the answer is
		// usually in hand before anybody presses play — and asked once per book,
		// because how a book was uploaded is not a per-chapter property.
		this.#probe = this.#probeChunked();

		window.addEventListener('pagehide', this.#onPageHide);
		document.addEventListener('visibilitychange', this.#onPageHide);
	}

	detach() {
		this.#detached = true;
		if (this.#handlers && this.#audio) {
			for (const [event, handler] of Object.entries(this.#handlers)) {
				this.#audio.removeEventListener(event, handler);
			}
		}
		this.#handlers = null;
		this.#releaseChunkUrl();
		this.#clearSleep();
		window.removeEventListener('pagehide', this.#onPageHide);
		document.removeEventListener('visibilitychange', this.#onPageHide);
	}

	#onPageHide = () => {
		if (document.visibilityState === 'hidden') this.#persist(true);
	};

	// -----------------------------------------------------------------------
	// Transport
	// -----------------------------------------------------------------------

	/**
	 * Move to a track. Playback only starts when `play` is true, so a restore
	 * or a manual track pick never fights the browser's autoplay policy.
	 * Picking any chapter is also an answer to the resume offer: it goes away.
	 */
	load(index, { play = false, seek = 0, keepResumeOffer = false } = {}) {
		const audio = this.#audio;
		if (!audio || index < 0 || index >= this.length) return;

		const track = this.trackAt(index);
		if (!track) {
			// The chapter sits in a window this page has not fetched. Ask for it
			// and re-enter once it lands, so every caller — a row tap, `next()`,
			// a restored position — waits the same way instead of stumbling on a
			// chapter that is merely on its way.
			this.pendingIndex = index;
			this.#awaitChapter(index, () => this.load(index, { play, seek, keepResumeOffer: true }));
			return;
		}

		this.pendingIndex = null;
		this.index = index;
		this.time = 0;
		this.duration = 0;
		this.buffered = 0;
		// Picking a chapter answers the resume offer; re-entering after a window
		// arrives does not, so the offer has to survive the wait.
		if (!keepResumeOffer) this.resumeOffer = null;
		this.#playSeconds = 0;
		this.#lastAudioTime = 0;
		this.#pendingSeek = seek > 0 ? seek : 0;
		// The chapters just ahead are what `next()` and the auto-advance want
		// next, so keeping them in hand costs nothing now and saves a stall at
		// the chapter boundary.
		this.chapters?.prefetch(index);

		if (this.#chunkedSupport === null && this.#probe && !this.#probeSettled) {
			// The one probe this book makes is still in flight. Waiting it out
			// costs a single small request and means the first chapter is already
			// playing from decoded chunks rather than being streamed plainly and
			// then switched under the listener.
			this.pendingIndex = index;
			this.#probe.then(() => {
				if (this.#detached) return;
				this.load(index, { play, seek, keepResumeOffer: true });
			});
			return;
		}

		if (this.#chunkedSupport === true) {
			// Decompressed here, one chunk at a time, and only as far as the
			// playhead needs. A chapter whose own manifest is missing plays
			// plainly: a book is chunked as a whole, but one chapter is not
			// evidence about the next.
			this.#loadChunked(track, { seek, play });
			return;
		}

		this.#chunks = null;
		this.#releaseChunkUrl();
		// Assigning `src` (rather than fetching) hands streaming to the browser:
		// it will Range-request the file and buffer incrementally.
		audio.src = track.url;
		audio.load();

		this.#updateMediaMetadata();
		this.#persist(true);

		if (play) {
			this.play();
		} else {
			this.playing = false;
		}
	}

	/**
	 * Wait for the window holding `index`, then run `next`.
	 *
	 * A window that never arrives clears the loading marker and leaves the
	 * engine where it was: the listener keeps the chapter they were on rather
	 * than a player stuck mid-load. `ensure` resolves rather than rejecting, so
	 * there is nothing to catch here.
	 */
	#awaitChapter(index, next) {
		const chapters = this.chapters;
		if (!chapters) {
			this.pendingIndex = null;
			return;
		}

		chapters.ensure(index).then((ready) => {
			if (this.#detached) return;
			if (!ready || !this.trackAt(index)) {
				if (this.pendingIndex === index) this.pendingIndex = null;
				return;
			}
			next();
		});
	}

	play() {
		const audio = this.#audio;
		if (!audio) return;
		if (!audio.src && this.current) {
			this.load(this.index, { play: true, seek: this.time });
			return;
		}

		const attempt = audio.play();
		if (attempt && typeof attempt.catch === 'function') {
			attempt.catch((e) => {
				// Only autoplay refusal means the browser blocked us. AbortError
				// means a newer load superseded this request (the element's own
				// events drive state from there), and decode/network failures
				// arrive via the `error` event — neither is "blocked".
				if (e?.name === 'NotAllowedError') {
					this.playing = false;
					this.interrupted = true;
				} else if (e?.name !== 'AbortError') {
					this.playing = false;
				}
			});
		}
	}

	pause() {
		this.#audio?.pause();
	}

	toggle() {
		if (this.playing) this.pause();
		else this.play();
	}

	next({ auto = false } = {}) {
		if (!this.hasNext) {
			this.pause();
			return;
		}
		this.load(this.index + 1, { play: true, seek: auto ? 0 : 0 });
	}

	previous() {
		if (!this.hasPrevious) {
			this.seek(0);
			return;
		}
		this.load(this.index - 1, { play: this.playing });
	}

	/** Jump to an absolute position in the current track. */
	seek(seconds) {
		const audio = this.#audio;
		if (!audio) return;

		const limit = this.displayDuration || 0;
		const target = Math.max(0, limit > 0 ? Math.min(seconds, limit) : seconds);
		this.time = target;

		if (this.#chunks) {
			// The bytes have to exist before the element can be put at them: a
			// position past the decompressed prefix would be clamped by the
			// element to the end of what it holds, which is a seek that silently
			// does nothing.
			this.#seekChunked(target);
			return;
		}

		if (Number.isFinite(audio.duration) && audio.duration > 0) {
			audio.currentTime = target;
		} else {
			// Metadata is not ready yet; apply the seek once it is.
			this.#pendingSeek = target;
		}
	}

	// -----------------------------------------------------------------------
	// Chunked chapters
	// -----------------------------------------------------------------------

	/**
	 * Asks whether this book is stored as chunks, and remembers the answer.
	 *
	 * A `404` is the ordinary answer for a book uploaded before chunking
	 * existed, so a missing table is not a failure and must not be logged or
	 * retried per chapter — it is the other half of the design. A transport
	 * failure is treated the same way: the plain stream is always available.
	 */
	async #probeChunked() {
		const track = this.current ?? this.tracks[0];
		if (!track) {
			// Nothing to ask about yet. The question stays open — a later load can
			// still find a chunked chapter — but nobody waits on it any more, so
			// that load goes plain instead of deferring to a settled probe.
			this.#probeSettled = true;
			return;
		}

		const table = await this.#chapterTable(track);
		this.#chunkedSupport = table ? true : false;
		this.#probeSettled = true;
	}

	/** One chapter's chunk table, or null when it has none. */
	async #chapterTable(track) {
		try {
			const response = await fetch(`${track.url}/manifest`);
			if (!response.ok) return null;
			return readChunkTable(await response.json());
		} catch {
			return null;
		}
	}

	/**
	 * Loads a chapter from its chunks and points the element at the Blob.
	 *
	 * Only as far as the requested position: replaying from the start loads one
	 * chunk, and a restored position loads exactly the chunks under it. A chapter
	 * that turns out to have no usable table — or a chunk that will not load —
	 * falls back to the plain URL, which is what played before chunks existed.
	 */
	async #loadChunked(track, { seek, play }) {
		const audio = this.#audio;
		if (!audio) return;

		const table = await this.#chapterTable(track);
		if (this.#detached || this.trackAt(this.index) !== track) return;

		if (!table) {
			this.#chunks = null;
			this.#releaseChunkUrl();
			audio.src = track.url;
			audio.load();
			this.#announceSource(play);
			return;
		}

		const duration = track.duration_seconds ?? 0;
		const offset = seek > 0 ? offsetForTime(seek, duration, table.rawSize) : 0;
		const source = new ChunkedAudio(table);

		try {
			await source.loadThroughOffset(offset);
		} catch {
			// A chunk that cannot be fetched or decoded is a chapter that plays
			// plainly, not a chapter that fails to play.
			if (this.#detached || this.trackAt(this.index) !== track) return;
			this.#chunks = null;
			this.#releaseChunkUrl();
			audio.src = track.url;
			audio.load();
			this.#announceSource(play);
			return;
		}

		if (this.#detached || this.trackAt(this.index) !== track) return;

		this.#chunks = source;
		this.#setChunkUrl(source.toBlobUrl());
		if (this.#chunkUrl) {
			audio.src = this.#chunkUrl;
			audio.load();
			// The Blob holds only the prefix that was decompressed, so a restored
			// position is applied once the element has read the new source.
			if (seek > 0) this.#pendingSeek = seek;
		} else {
			audio.src = track.url;
			audio.load();
		}

		this.#announceSource(play);
	}

	/**
	 * The bookkeeping every source change owes, wherever the bytes came from:
	 * the Media Session's idea of the chapter, the saved session, and whether it
	 * should be playing. The plain path does the same at the end of `load`.
	 */
	#announceSource(play) {
		this.#updateMediaMetadata();
		this.#persist(true);
		if (play) this.play();
		else this.playing = false;
	}

	/**
	 * Moves a chunked chapter's playhead, extending the prefix if it has to.
	 *
	 * A position already inside the decompressed bytes is a plain assignment and
	 * no network at all; a position past them decompresses the chunks in between
	 * first and re-points `src`, because a bigger Blob is a different URL.
	 */
	async #seekChunked(target) {
		const chunks = this.#chunks;
		const audio = this.#audio;
		if (!chunks || !audio) return;

		const before = chunks.loadedBytes;
		try {
			await chunks.loadThroughTime(target, this.displayDuration);
		} catch {
			return;
		}

		if (this.#detached || chunks !== this.#chunks) return;

		if (chunks.loadedBytes === before) {
			audio.currentTime = target;
			return;
		}

		this.#setChunkUrl(chunks.toBlobUrl());
		if (!this.#chunkUrl) return;

		audio.src = this.#chunkUrl;
		audio.load();
		this.#pendingSeek = target;
	}

	/** Swaps in a Blob URL, revoking the one it replaces. */
	#setChunkUrl(url) {
		if (!url) return;
		if (this.#chunkUrl) URL.revokeObjectURL(this.#chunkUrl);
		this.#chunkUrl = url;
	}

	/** Drops the Blob URL, if any. A revoked URL is a Blob that can be collected. */
	#releaseChunkUrl() {
		if (this.#chunkUrl) {
			URL.revokeObjectURL(this.#chunkUrl);
			this.#chunkUrl = null;
		}
	}

	/** Seek by a ratio (0..1) of the current track; used by the scrubber. */
	seekToRatio(ratio) {
		const limit = this.displayDuration;
		if (!limit) return;
		this.seek(Math.min(Math.max(ratio, 0), 1) * limit);
	}

	/** Skip forward (positive) or backward (negative) by seconds. */
	skip(delta) {
		const base = this.#audio?.currentTime ?? this.time;
		this.seek(base + delta);
	}

	setRate(value) {
		const next = PLAYBACK_RATES.includes(value) ? value : 1;
		this.rate = next;
		if (this.#audio) this.#audio.playbackRate = next;
		this.#persist(true);
	}

	cycleRate() {
		const at = PLAYBACK_RATES.indexOf(this.rate);
		this.setRate(PLAYBACK_RATES[(at + 1) % PLAYBACK_RATES.length]);
	}

	setVolume(value) {
		const next = Math.min(Math.max(value, 0), 1);
		this.volume = next;
		if (this.#audio) {
			this.#audio.volume = next;
			// Adjusting the slider is an explicit intent to hear something.
			if (next > 0 && this.#audio.muted) this.#audio.muted = false;
		}
		this.#persist(true);
	}

	toggleMute() {
		if (!this.#audio) return;
		this.#audio.muted = !this.#audio.muted;
	}

	setSkipSeconds(value) {
		const next = Math.min(Math.max(Math.round(value), 5), 60);
		this.skipSeconds = next;
		this.#persist(true);
	}

	// -----------------------------------------------------------------------
	// Sleep timer
	// -----------------------------------------------------------------------

	/** Pause after `minutes`, or at the end of the current chapter for 'chapter'. */
	startSleep(mode) {
		this.#clearSleep();

		if (mode === 'chapter') {
			this.sleepMode = 'chapter';
			this.sleepRemaining = 0;
			return;
		}

		const minutes = Number(mode);
		if (!Number.isFinite(minutes) || minutes <= 0) return;

		this.sleepMode = minutes;
		this.#sleepDeadline = Date.now() + minutes * 60_000;
		this.sleepRemaining = minutes * 60;

		this.#sleepTicker = setInterval(() => {
			const remaining = Math.max(0, Math.round((this.#sleepDeadline - Date.now()) / 1000));
			this.sleepRemaining = remaining;
			if (remaining <= 0) {
				this.#clearSleep();
				this.pause();
			}
		}, 1000);
	}

	cancelSleep() {
		this.#clearSleep();
	}

	#clearSleep() {
		if (this.#sleepTicker) {
			clearInterval(this.#sleepTicker);
			this.#sleepTicker = null;
		}
		this.sleepMode = null;
		this.sleepRemaining = 0;
	}

	// -----------------------------------------------------------------------
	// Element event handling
	// -----------------------------------------------------------------------

	#onTimeUpdate() {
		const audio = this.#audio;
		if (!audio) return;
		// Advance the play counter by real elapsed listening: only while
		// actually playing (a paused seek fires timeupdate too), and only for
		// small deltas — a large jump is a seek, not listening.
		//
		// Audio time is converted to the listener's own seconds by dividing by
		// the playback rate, because the counter measures how long somebody
		// listened, not how much of the book went past: at 2x a chapter reaches
		// its next play in ten real seconds, not five.
		const delta = audio.currentTime - this.#lastAudioTime;
		this.#lastAudioTime = audio.currentTime;
		const listened = delta / this.rate;
		if (this.playing && listened > 0 && listened < 5) this.#accumulatePlay(listened);
		this.time = audio.currentTime;
		this.#onProgress();
		this.#persist(false);
		this.#updateMediaPosition();
	}

	/**
	 * Report a play for every PLAY_REPORT_INTERVAL_SECONDS of genuine listening.
	 *
	 * There is no once-per-chapter cap: the figure is a measure of time
	 * listened, so a listener who stays in a chapter for an hour reports 360
	 * times. The remainder is carried over rather than discarded, which keeps
	 * the cadence honest instead of letting it drift late. Fires through the
	 * engine's meta so it keeps working while the mini player continues playback
	 * on pages whose full player is long gone.
	 */
	#accumulatePlay(delta) {
		const track = this.current;
		if (!track) return;
		this.#playSeconds += delta;
		while (this.#playSeconds >= PLAY_REPORT_INTERVAL_SECONDS) {
			this.#playSeconds -= PLAY_REPORT_INTERVAL_SECONDS;
			try {
				this.#meta.onTrackPlayed?.(track.id);
			} catch {
				// A failing beacon must never take playback down with it.
			}
		}
	}

	#onLoadedMetadata() {
		const audio = this.#audio;
		if (!audio) return;

		if (Number.isFinite(audio.duration) && audio.duration > 0) {
			this.duration = audio.duration;
		}

		if (this.#pendingSeek > 0) {
			const limit = this.displayDuration || this.#pendingSeek;
			const target = Math.min(this.#pendingSeek, Math.max(limit - 1, 0));
			this.#pendingSeek = 0;
			if (target > 0) {
				audio.currentTime = target;
				this.time = target;
			}
		}

		this.#updateMediaPosition();
	}

	#onProgress() {
		const audio = this.#audio;
		if (!audio || audio.buffered.length === 0) return;
		// Show how far ahead the browser has buffered, which makes the
		// streaming behaviour visible rather than mysterious.
		this.buffered = audio.buffered.end(audio.buffered.length - 1);
	}

	#onEnded() {
		// A chapter-scoped sleep timer stops the session here instead of rolling
		// into the next chapter.
		if (this.sleepMode === 'chapter') {
			this.#clearSleep();
			this.playing = false;
			this.#persist(true);
			return;
		}

		if (this.hasNext) {
			this.next({ auto: true });
			return;
		}

		// End of the playlist: stop cleanly and leave the position at the end.
		this.playing = false;
		this.time = this.displayDuration;
		this.#persist(true);
	}

	#onError() {
		const audio = this.#audio;
		const track = this.current;
		if (!audio || !track) return;

		const code = audio.error?.code;
		// Stable keys rather than prose: the rendering component picks the
		// message matching the book's language.
		const key =
			code === 1
				? 'aborted'
				: code === 2
					? 'network'
					: code === 3
						? 'decode'
						: code === 4
							? 'unsupported'
							: 'generic';

		this.failures = { ...this.failures, [track.id]: key };

		// One unplayable file must not stall the whole book: move on if there is
		// somewhere to go, otherwise stop.
		if (this.hasNext) {
			this.load(this.index + 1, { play: true });
		} else {
			this.playing = false;
		}
	}

	// -----------------------------------------------------------------------
	// Persistence
	// -----------------------------------------------------------------------

	#persist(force) {
		if (typeof localStorage === 'undefined') return;
		const now = Date.now();
		if (!force && now - this.#lastPersist < PERSIST_INTERVAL_MS) return;
		this.#lastPersist = now;

		const track = this.current;
		try {
			localStorage.setItem(
				this.#storageKey,
				JSON.stringify({
					index: this.index,
					trackId: track?.id ?? null,
					time: this.time,
					rate: this.rate,
					volume: this.volume,
					muted: this.muted,
					skipSeconds: this.skipSeconds,
					updatedAt: now
				})
			);
		} catch {
			// Private browsing or a full quota: persistence is a convenience, not
			// a requirement, so failing to save must never break playback.
		}
	}

	#restore() {
		if (typeof localStorage === 'undefined') return;

		let saved = null;
		try {
			const raw = localStorage.getItem(this.#storageKey);
			if (raw) saved = JSON.parse(raw);
		} catch {
			saved = null;
		}
		if (!saved || typeof saved !== 'object') return;

		if (typeof saved.rate === 'number' && PLAYBACK_RATES.includes(saved.rate)) {
			this.rate = saved.rate;
		}
		if (typeof saved.volume === 'number') {
			this.volume = Math.min(Math.max(saved.volume, 0), 1);
		}
		if (typeof saved.muted === 'boolean') this.muted = saved.muted;
		if (typeof saved.skipSeconds === 'number') this.skipSeconds = saved.skipSeconds;

		if (this.#audio) {
			this.#audio.playbackRate = this.rate;
			this.#audio.volume = this.volume;
			this.#audio.muted = this.muted;
		}

		// Prefer the track id: the playlist may have been reordered or had
		// tracks inserted since the position was saved. Windowed chapters add a
		// second reason to keep the saved index — the track itself may simply
		// not have arrived yet, and `load` fetches the window it sits in.
		let index = this.tracks.findIndex((t) => t?.id === saved.trackId);
		if (index < 0) {
			index = Number.isInteger(saved.index) ? saved.index : 0;
		}
		if (index < 0 || index >= this.length) index = 0;

		const time = Number.isFinite(saved.time) && saved.time > 0 ? saved.time : 0;
		const track = this.trackAt(index);
		const nearEnd = track?.duration_seconds ? time > track.duration_seconds - 5 : false;

		// Load the saved track so the metadata and duration are ready, but do not
		// auto-play: browsers block audible autoplay without a gesture, and
		// silently starting mid-chapter would be jarring anyway.
		this.load(index, { play: false, seek: 0 });

		if (time > 5 && !nearEnd) {
			this.resumeOffer = { index, time, trackId: track?.id ?? null };
		}
	}

	/** Accept the offered resume position. */
	acceptResume() {
		if (!this.resumeOffer) return;
		const { time } = this.resumeOffer;
		this.resumeOffer = null;
		this.seek(time);
		this.play();
	}

	dismissResume() {
		this.resumeOffer = null;
		this.#persist(true);
	}

	/** Forget the saved position and start the book from the beginning. */
	clearSaved() {
		if (typeof localStorage !== 'undefined') {
			try {
				localStorage.removeItem(this.#storageKey);
			} catch {
				// Ignore: nothing to clean up if storage is unavailable.
			}
		}
		this.resumeOffer = null;
		this.load(0, { play: false });
	}

	// -----------------------------------------------------------------------
	// OS media integration
	// -----------------------------------------------------------------------

	/**
	 * Install the OS media-session handlers for this engine.
	 *
	 * Public because the handlers are global per document: when another engine
	 * attaches (the reader opened a second audiobook without playing it), the
	 * one that is actually playing has to re-claim them.
	 */
	setupMediaSession() {
		if (typeof navigator === 'undefined' || !('mediaSession' in navigator)) return;

		const session = navigator.mediaSession;
		const set = (action, handler) => {
			try {
				session.setActionHandler(action, handler);
			} catch {
				// Not every action is supported by every browser.
			}
		};

		set('play', () => this.play());
		set('pause', () => this.pause());
		set('stop', () => this.pause());
		set('previoustrack', () => this.previous());
		set('nexttrack', () => this.next());
		set('seekbackward', (details) => this.skip(-(details?.seekOffset ?? this.skipSeconds)));
		set('seekforward', (details) => this.skip(details?.seekOffset ?? this.skipSeconds));
		set('seekto', (details) => {
			if (typeof details?.seekTime === 'number') this.seek(details.seekTime);
		});

		this.#updateMediaMetadata();
	}

	#updateMediaMetadata() {
		if (typeof navigator === 'undefined' || !('mediaSession' in navigator)) return;
		const track = this.current;
		if (!track) return;

		const meta = this.#meta ?? {};
		const artwork = meta.coverUrl
			? [{ src: meta.coverUrl, sizes: '512x512', type: 'image/webp' }]
			: [];

		try {
			navigator.mediaSession.metadata = new MediaMetadata({
				title: track.title,
				artist: meta.translator || meta.author || 'Audiobook',
				album: meta.title || 'Audiobook',
				artwork
			});
		} catch {
			// MediaMetadata is unavailable in some embedded browsers.
		}
	}

	#updateMediaPosition() {
		if (typeof navigator === 'undefined' || !('mediaSession' in navigator)) return;
		const duration = this.displayDuration;
		if (!duration || !Number.isFinite(this.time)) return;

		try {
			navigator.mediaSession.setPositionState({
				duration,
				position: Math.min(Math.max(this.time, 0), duration),
				playbackRate: this.rate
			});
		} catch {
			// Throws if the values are momentarily inconsistent; the next tick
			// will publish a valid state.
		}
	}

	/** Extra context (title, author, translator, cover) for OS metadata. */
	setMeta(meta) {
		this.#meta = meta ?? {};
		this.#updateMediaMetadata();
	}

	#meta = {};
}
