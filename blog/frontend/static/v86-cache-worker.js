const CACHE_NAME = 'v86-runtime-v4';

const isV86ImmutableAsset = (pathname) => {
	if (pathname.includes('/v86/assets/systems/')) return true;
	// Snapshots are content-addressed by the sha256 of the compressed state, so
	// a changed state always arrives under a new path and can be cached
	// indefinitely. Caching matters most here: unlike the disks, a state cannot
	// be range-loaded, so an uncached visit re-downloads the whole blob.
	if (pathname.includes('/v86/snapshots/')) return pathname.endsWith('/state.zst');
	if (!pathname.includes('/projects/s/') || !pathname.includes('/v86/')) return false;
	return pathname.endsWith('/full.iso');
};

self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (event) => {
	event.waitUntil(
		(async () => {
			const keys = await caches.keys();
			await Promise.all(keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key)));
			await self.clients.claim();
		})()
	);
});

self.addEventListener('fetch', (event) => {
	if (event.request.method !== 'GET') return;
	const url = new URL(event.request.url);
	if (!isV86ImmutableAsset(url.pathname)) return;
	event.respondWith(
		caches.open(CACHE_NAME).then(async (cache) => {
			const cached = await cache.match(event.request);
			if (cached) return cached;
			const response = await fetch(event.request);
			if (response.ok) {
				try {
					await cache.put(event.request, response.clone());
				} catch (error) {
					console.error('v86 cache put failed:', error);
				}
			}
			return response;
		})
	);
});
