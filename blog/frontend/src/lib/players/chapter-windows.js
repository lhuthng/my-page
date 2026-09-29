/**
 * Window arithmetic for an audiobook's chapter list.
 *
 * The players no longer receive a whole book's chapters in one payload: they
 * ask for a window at a time. That leaves a small handful of questions — which
 * window an index belongs to, and what the list should render while some
 * windows are still in flight — and those are pure, so they live here where
 * they can be tested without a browser, an audio element or a book.
 */

/** Chapters per fetch. Sized to a screen or two of list. */
export const CHAPTER_WINDOW = 20;
/**
 * Chapters ahead of the playhead to have in hand before they are needed.
 *
 * Kept below the window size on purpose: while the playhead is more than this
 * many chapters from the end of its window, the prefetch lands inside a window
 * already loaded and costs nothing. It only reaches for the next window as the
 * listener gets close to the boundary.
 */
export const PREFETCH_AHEAD = 3;

/** Start index of the window that contains `index`. */
export function windowStart(index, size = CHAPTER_WINDOW) {
	return Math.max(0, Math.floor(index / size) * size);
}

/** Every window start covering a book of `total` chapters, in order. */
export function windowStarts(total, size = CHAPTER_WINDOW) {
	const starts = [];
	for (let start = 0; start < total; start += size) starts.push(start);
	return starts;
}

/** The chapters a window holds, clamped to the end of the book. */
export function windowCount(start, total, size = CHAPTER_WINDOW) {
	return Math.max(0, Math.min(size, total - start));
}

/**
 * What the chapter list renders right now.
 *
 * One item per loaded chapter, one skeleton item per window already being
 * fetched, and one gap item per stretch of chapters nobody has asked for yet.
 * A gap is not dead space: it is the row the reader scrolls to, and reaching it
 * is what fetches that window — so a 2000-chapter book paints twenty items
 * rather than two thousand, and walking down it loads the book a window at a
 * time.
 */
export function planRows({ total, loaded = [], pending = [], size = CHAPTER_WINDOW }) {
	const loadedSet = new Set(loaded);
	const pendingSet = new Set(pending);
	const rows = [];

	for (let start = 0; start < total; start += size) {
		const count = windowCount(start, total, size);

		if (loadedSet.has(start)) {
			for (let index = start; index < start + count; index += 1) {
				rows.push({ kind: 'chapter', index });
			}
			continue;
		}

		// Adjacent windows of the same kind are one row: a stretch nobody has
		// fetched is a single reachable gap however many windows it spans, and a
		// run of windows in flight is one skeleton block.
		const kind = pendingSet.has(start) ? 'skeleton' : 'gap';
		const last = rows.at(-1);
		if (last && last.kind === kind && last.start + last.count === start) {
			last.count += count;
		} else {
			rows.push({ kind, start, count });
		}
	}

	return rows;
}

/** Windows still missing from `loaded`, in order. */
export function missingWindows(total, loaded = [], size = CHAPTER_WINDOW) {
	const loadedSet = new Set(loaded);
	return windowStarts(total, size).filter((start) => !loadedSet.has(start));
}
