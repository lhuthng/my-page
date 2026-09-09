// Shared helpers for the v86 offline cache (Cache API entries written by
// static/v86-cache-worker.js). CACHE_NAME must stay in sync with the worker —
// it is a classic worker and cannot import this module.

export const CACHE_NAME = 'v86-runtime-v4';
export const FETCH_CONCURRENCY = 8;

// A disk's chunks live next to the ".img.zst" sentinel file v86 points at:
// .../6bf9b136.../.img.zst -> .../6bf9b136.../0-262144.img.zst
export function chunkPrefix(url) {
	return url.replace(/\.img\.zst$/, '');
}

// Part names always use chunk-size steps, even for the final short chunk —
// this mirrors both v86's partfile loader and the backend's disk_part_name.
export function chunkUrl(prefix, index, chunkSize) {
	const start = index * chunkSize;
	return `${prefix}${start}-${start + chunkSize}.img.zst`;
}

export function formatBytes(bytes) {
	if (!bytes || bytes <= 0) return '0 MB';
	const mb = bytes / 1048576;
	if (mb >= 1024) return `${(mb / 1024).toFixed(2)} GB`;
	if (mb >= 1) return `${Math.round(mb)} MB`;
	return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}

function cachedPathSet() {
	if (typeof caches === 'undefined') return Promise.resolve(null);
	return caches
		.open(CACHE_NAME)
		.then((cache) => cache.keys())
		.then((keys) => new Set(keys.map((request) => new URL(request.url).pathname)))
		.catch(() => null);
}

// How much of one disk is already cached: chunk count and estimated bytes.
// Chunks are matched by URL path, so entries land regardless of which origin
// (backend or CDN) the runtime points at.
export async function measureDiskChunks({ url, size, chunkSize }) {
	const total = Math.max(1, Math.ceil(size / chunkSize));
	const prefix = new URL(chunkPrefix(url), window.location.origin).pathname;
	const have = await cachedPathSet();
	let count = 0;
	if (have) {
		for (let i = 0; i < total; i++) {
			if (have.has(chunkUrl(prefix, i, chunkSize))) count++;
		}
	}
	// Compressed chunk sizes vary; scaling the disk size by the cached share
	// is the right estimate and stays 0 while nothing is cached.
	return { count, total, bytes: Math.round((size * count) / total) };
}

// Fetch every still-missing chunk of one disk; the service worker intercepts
// these and caches them. onProgress fires after each batch with the number of
// chunks that landed since the last call, so the UI can tick live. The abort
// signal fences between batches; in-flight fetches reject and are swallowed.
export async function fetchDiskChunks({ url, size, chunkSize }, signal, onProgress) {
	const prefix = new URL(chunkPrefix(url), window.location.origin).href;
	const have = await cachedPathSet();
	const total = Math.max(1, Math.ceil(size / chunkSize));
	const missing = [];
	if (have) {
		for (let i = 0; i < total; i++) {
			if (!have.has(chunkUrl(prefix, i, chunkSize))) missing.push(i);
		}
	} else {
		for (let i = 0; i < total; i++) missing.push(i);
	}
	let landed = 0;
	for (let at = 0; at < missing.length; at += FETCH_CONCURRENCY) {
		if (signal?.aborted) break;
		await Promise.all(
			missing.slice(at, at + FETCH_CONCURRENCY).map(async (i) => {
				try {
					const response = await fetch(chunkUrl(prefix, i, chunkSize), { signal });
					if (response.ok) landed++;
				} catch {
					// Aborted or failed; retried on the next run.
				}
			})
		);
		onProgress?.(landed);
		landed = 0;
	}
}

// Delete every cached chunk belonging to one disk — this is what actually
// frees the user's storage. Matching goes by URL path so it hits keys cached
// under whichever origin (backend or CDN) the runtime points at; deleting a
// Request built on the page origin would silently miss those.
export async function deleteDiskChunks({ url, size, chunkSize }) {
	if (typeof caches === 'undefined') return;
	const cache = await caches.open(CACHE_NAME).catch(() => null);
	if (!cache) return;
	const prefix = new URL(chunkPrefix(url), window.location.origin).pathname;
	const total = Math.max(1, Math.ceil(size / chunkSize));
	const wanted = new Set(Array.from({ length: total }, (_, i) => chunkUrl(prefix, i, chunkSize)));
	await Promise.all(
		(await cache.keys())
			.filter((request) => wanted.has(new URL(request.url).pathname))
			.map((request) => cache.delete(request))
	);
}

export async function clearV86Cache() {
	if (typeof caches === 'undefined') return;
	await caches.delete(CACHE_NAME);
}

// Total size of everything the offline cache keeps. Content-length is always
// set on these responses (the backend and R2 both set it), so matching every
// entry is enough; no body reads.
export async function measureV86Cache() {
	if (typeof caches === 'undefined') return { bytes: 0, count: 0 };
	const cache = await caches.open(CACHE_NAME).catch(() => null);
	if (!cache) return { bytes: 0, count: 0 };
	const keys = await cache.keys();
	let bytes = 0;
	for (const request of keys) {
		const response = await cache.match(request).catch(() => null);
		bytes += Number(response?.headers.get('content-length') ?? 0);
	}
	return { bytes, count: keys.length };
}
