/**
 * Playback engine for an audiobook.
 *
 * Deliberately owns no markup: it drives a single `<audio>` element and exposes
 * reactive state, so the Svelte component only renders controls. Mirrors the
 * `players/*.svelte.js` convention used by the game players.
 *
 * Streaming: the element's `src` is pointed at the track's media URL and never
 * at a blob or a fetched ArrayBuffer. The browser therefore issues ordinary
 * HTTP requests, using Range requests to buffer ahead and to seek, so a
 * multi-hour file is never held in memory. Only one track is ever loading.
 */

const STORAGE_PREFIX = 'audiobook-player:';
/** Writing localStorage on every `timeupdate` would fire ~4x/second. */
const PERSIST_INTERVAL_MS = 4000;
const PLAYBACK_RATES = [0.75, 1, 1.25, 1.5, 1.75, 2];
const SLEEP_PRESETS_MINUTES = [15, 30, 45, 60];

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
	/** trackId -> error message, for tracks the browser could not load. */
	failures = $state({});
	/** null | minutes (number) | 'chapter' */
	sleepMode = $state(null);
	/** Seconds left on a timed sleep; 0 when no timer is running. */
	sleepRemaining = $state(0);
	/** A saved position was found and is offered as a resume. */
	resumeOffer = $state(null);

	#audio = null;
	#storageKey = null;
	#lastPersist = 0;
	#sleepTicker = null;
	#sleepDeadline = 0;
	#pendingSeek = 0;
	#handlers = null;
	#detached = false;

	constructor(tracks, { storageKey = 'default', skipSeconds = 15 } = {}) {
		this.tracks = tracks ?? [];
		this.#storageKey = STORAGE_PREFIX + storageKey;
		this.skipSeconds = skipSeconds;
	}

	get current() {
		return this.tracks[this.index] ?? null;
	}

	get hasNext() {
		return this.index < this.tracks.length - 1;
	}

	get hasPrevious() {
		return this.index > 0;
	}

	get totalDuration() {
		return this.tracks.reduce((sum, track) => sum + (track.duration_seconds ?? 0), 0);
	}

	/** Duration to display: the live element value, else the stored metadata. */
	get displayDuration() {
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
	 */
	load(index, { play = false, seek = 0 } = {}) {
		const audio = this.#audio;
		if (!audio || index < 0 || index >= this.tracks.length) return;

		const track = this.tracks[index];
		this.index = index;
		this.time = 0;
		this.duration = 0;
		this.buffered = 0;
		this.#pendingSeek = seek > 0 ? seek : 0;

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

		if (Number.isFinite(audio.duration) && audio.duration > 0) {
			audio.currentTime = target;
		} else {
			// Metadata is not ready yet; apply the seek once it is.
			this.#pendingSeek = target;
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
		this.time = audio.currentTime;
		this.#onProgress();
		this.#persist(false);
		this.#updateMediaPosition();
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
		const message =
			code === 1
				? 'Playback aborted.'
				: code === 2
					? 'Network error while loading this track.'
					: code === 3
						? 'This track could not be decoded.'
						: code === 4
							? 'This audio format is not supported.'
							: 'This track could not be played.';

		this.failures = { ...this.failures, [track.id]: message };

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
		// tracks inserted since the position was saved.
		let index = this.tracks.findIndex((t) => t.id === saved.trackId);
		if (index < 0) {
			index = Number.isInteger(saved.index) ? saved.index : 0;
		}
		if (index < 0 || index >= this.tracks.length) index = 0;

		const time = Number.isFinite(saved.time) && saved.time > 0 ? saved.time : 0;
		const track = this.tracks[index];
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
