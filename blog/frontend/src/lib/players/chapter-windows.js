export const CHAPTER_WINDOW = 20;
export const PREFETCH_AHEAD = 3;

export function windowStart(index, size = CHAPTER_WINDOW) {
	return Math.max(0, Math.floor(index / size) * size);
}

export function windowStarts(total, size = CHAPTER_WINDOW) {
	const starts = [];
	for (let start = 0; start < total; start += size) starts.push(start);
	return starts;
}

export function windowCount(start, total, size = CHAPTER_WINDOW) {
	return Math.max(0, Math.min(size, total - start));
}

export function missingWindows(total, loaded = [], size = CHAPTER_WINDOW) {
	const loadedSet = new Set(loaded);
	return windowStarts(total, size).filter((start) => !loadedSet.has(start));
}

export function pageCount(total, size = CHAPTER_WINDOW) {
	return total > 0 ? Math.ceil(total / size) : 0;
}

export function clampPage(page, total, size = CHAPTER_WINDOW) {
	const last = Math.max(1, pageCount(total, size));
	return Math.min(last, Math.max(1, Number.isFinite(page) ? Math.trunc(page) : 1));
}

export function pageStart(page, size = CHAPTER_WINDOW) {
	return Math.max(0, (Math.trunc(page) - 1) * size);
}

export function parseChapterNumber(value) {
	const text = String(value ?? '').trim();
	if (!/^\d+$/.test(text)) return null;
	const number = Number(text);
	return Number.isSafeInteger(number) && number > 0 ? number : null;
}

export function validateChapterJump(audiobook, number, size = CHAPTER_WINDOW) {
	if (!audiobook || !Array.isArray(audiobook.tracks)) return null;
	if (!Number.isSafeInteger(number) || number < 1) return null;
	if (!Number.isSafeInteger(size) || size < 1) return null;

	const { tracks, tracks_offset: offset, track_count: total } = audiobook;
	if (!Number.isSafeInteger(offset) || offset < 0 || offset % size !== 0) return null;
	if (!Number.isSafeInteger(total) || total < 0) return null;
	if (tracks.length !== windowCount(offset, total, size)) return null;

	const localIndex = tracks.findIndex((track) => track?.number === number);
	if (localIndex < 0 || offset + localIndex >= total) return null;
	return { offset, ordinal: offset + localIndex, total };
}
