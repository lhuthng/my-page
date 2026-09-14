import { fixClientRoute, route } from '$lib/server/proxy.js';

export async function load({ fetch, url, setHeaders }) {
	const params = new URLSearchParams();
	const tag = url.searchParams.get('tag');
	const term = url.searchParams.get('term');

	if (tag) params.set('tag', tag);
	if (term) params.set('term', term);
	params.set('limit', '48');
	params.set('offset', '0');

	const res = await fetch(route(`audiobooks/public/all?${params}`));

	if (!res.ok) {
		console.error('audiobooks: backend returned', res.status, await res.text());
		return { audiobooks: [], tag: tag ?? null, term: term ?? null };
	}

	setHeaders({
		'cache-control': 'public, max-age=60, s-maxage=60'
	});

	const { audiobooks } = await res.json();

	// Re-root cover URLs so the browser can fetch media straight from the
	// backend origin (or the /api proxy) instead of this SvelteKit server.
	for (const audiobook of audiobooks) {
		audiobook.url = fixClientRoute(audiobook.url);
	}

	return { audiobooks, tag: tag ?? null, term: term ?? null };
}
