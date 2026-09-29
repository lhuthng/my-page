/**
 * One audiobook's chapter list, fetched a window at a time.
 *
 * The public detail endpoint returns a window of chapters rather than the whole
 * book, and this is the client half of that: it knows how many chapters there
 * are, keeps the ones that have arrived, and hands out the reachable-gap view
 * the chapter list renders. Everything about *which* chapters exist and *where*
 * they sit comes from `chapter-windows.js`; what lives here is the fetching and
 * the reactive bookkeeping.
 *
 * A window that fails to arrive is not an error state the listener has to
 * confront: `ensure` resolves `false`, the chapter stays a gap, and scrolling
 * to it (or playing into it) asks again. Losing one request must never take
 * playback down, so nothing here throws.
 */
import {
	CHAPTER_WINDOW,
	PREFETCH_AHEAD,
	missingWindows,
	planRows,
	windowStart
} from './chapter-windows.js';
import { fixUrl } from '$lib/utils/media-path.js';

export class AudiobookChapters {
	/** Chapters in the book, known from the first response. */
	total = $state(0);
	/**
	 * The playlist, one slot per chapter, holes where a chapter has not been
	 * fetched yet. Not reactive on purpose: the reactive facts about it are
	 * `loaded` and `pending`, and the engine reads slots imperatively.
	 */
	tracks = [];
	/** Window starts already in `tracks`. */
	loaded = $state([]);
	/** Window starts being fetched right now. */
	pending = $state([]);

	#slug;
	#base;
	#size;
	#inflight = new Map();

	constructor({ slug, total = 0, size = CHAPTER_WINDOW, base = '/api' }) {
		this.#slug = slug;
		this.#base = base;
		this.#size = size;
		this.total = total;
		this.tracks = new Array(Math.max(0, total));
	}

	get size() {
		return this.#size;
	}

	/** Every chapter of the book is in `tracks`. */
	get complete() {
		return this.total > 0 && missingWindows(this.total, this.loaded, this.#size).length === 0;
	}

	/** What the chapter list renders: chapters, skeletons, and reachable gaps. */
	get rows() {
		return planRows({
			total: this.total,
			loaded: this.loaded,
			pending: this.pending,
			size: this.#size
		});
	}

	trackAt(index) {
		return this.tracks[index] ?? null;
	}

	/** Install a window that arrived with the page, without a second request. */
	seed(start, list, total = this.total) {
		this.#install(start, list ?? [], total);
	}

	/**
	 * The window holding `index`, fetching it if it has not arrived.
	 *
	 * Resolves `true` once the chapter is playable and `false` if the request
	 * failed, so a caller can tell "wait a moment" from "this is not coming".
	 * Concurrent asks for one window share a single request.
	 */
	ensure(index) {
		if (this.total <= 0) return Promise.resolve(false);

		const start = windowStart(index, this.#size);
		if (this.loaded.includes(start)) return Promise.resolve(true);

		const inflight = this.#inflight.get(start);
		if (inflight) return inflight;

		this.pending = [...this.pending, start];
		const request = this.#fetchWindow(start).finally(() => {
			this.#inflight.delete(start);
			this.pending = this.pending.filter((started) => started !== start);
		});
		this.#inflight.set(start, request);
		return request;
	}

	/**
	 * Every window still missing. A search or a reversed list is browsing the
	 * book as a whole, which is exactly what the windowed list otherwise avoids
	 * doing, so those two ask for the rest deliberately.
	 */
	ensureAll() {
		if (this.total <= 0) return Promise.resolve();
		const wanted = missingWindows(this.total, this.loaded, this.#size);
		return Promise.all(wanted.map((start) => this.ensure(start)));
	}

	/**
	 * Keep the chapters just ahead of the playhead in hand. Called as the
	 * playhead moves; while it is more than a few chapters from the end of its
	 * window this is free, and near the boundary it pulls the next one in
	 * before the listener gets there.
	 */
	prefetch(index) {
		const ahead = index + PREFETCH_AHEAD;
		if (this.total <= 0 || ahead >= this.total) return;
		this.ensure(ahead);
	}

	async #fetchWindow(start) {
		try {
			const response = await fetch(
				`${this.#base}/audiobooks/public/s/${encodeURIComponent(this.#slug)}` +
					`?tracks_offset=${start}&tracks_limit=${this.#size}`
			);
			if (!response.ok) return false;
			const { audiobook } = await response.json();
			const tracks = audiobook?.tracks ?? [];

			if (!Number.isFinite(audiobook?.track_count)) {
				// A backend that predates windowed chapters ignores the window and
				// answers with the whole book. It is taken from the top as one
				// answer: installing it at the offset it was asked for would shift
				// every chapter, and a whole-book answer covers every window, so
				// nothing is left to fetch either.
				if (tracks.length === 0) return false;
				this.#install(0, tracks, tracks.length);
				return true;
			}

			this.#install(start, tracks, audiobook.track_count);
			return true;
		} catch {
			return false;
		}
	}

	#install(start, list, total) {
		if (Number.isFinite(total) && total >= 0 && total !== this.total) {
			// A chapter was added or removed since the page was rendered; the
			// playlist is grown or trimmed to match, keeping what has arrived.
			this.total = total;
			this.tracks.length = total;
		}

		list.forEach((track, offset) => {
			const index = start + offset;
			if (index >= this.total) return;
			// Backend-relative (`media/i/<short_name>`) on the way in, browser-
			// fetchable on the way to the <audio> element.
			this.tracks[index] = { ...track, url: fixUrl(track.url) };
		});

		// Every window this answer covers counts as loaded — normally exactly one,
		// but a short book or a whole-book answer covers several, and a window
		// only counts when the answer fills it to its end.
		const covered = start + list.length;
		const loaded = new Set(this.loaded);
		for (let from = windowStart(start, this.#size); from < covered; from += this.#size) {
			const end = Math.min(from + this.#size, this.total);
			if (end > from && end <= covered) loaded.add(from);
		}
		if (loaded.size !== this.loaded.length) {
			this.loaded = [...loaded].sort((a, b) => a - b);
		}
	}
}
