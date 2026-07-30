const BASE_CACHE = 'v86-base-parts-v1';

self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (event) => event.waitUntil(self.clients.claim()));

self.addEventListener('fetch', (event) => {
	const url = new URL(event.request.url);
	if (event.request.method !== 'GET' || !url.pathname.includes('/v86/assets/systems/')) {
		return;
	}
	event.respondWith(
		caches.open(BASE_CACHE).then(async (cache) => {
			const cached = await cache.match(event.request);
			if (cached) return cached;
			const response = await fetch(event.request);
			if (response.ok) await cache.put(event.request, response.clone());
			return response;
		})
	);
});
