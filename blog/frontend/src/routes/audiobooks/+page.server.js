import { fixClientRoute, route } from '$lib/server/proxy.js';

export async function load({ fetch, url, setHeaders }) {
	const firstOffset = 12;
	const tag = url.searchParams.get('tag');
	const term = url.searchParams.get('term');

	const params = new URLSearchParams();
	if (tag) params.set('tag', tag);
	if (term) params.set('term', term);
	params.set('limit', String(firstOffset));
	params.set('offset', '0');

	const res = await fetch(route(`audiobooks/public/all?${params}`));

	if (!res.ok) {
		console.error('audiobooks: backend returned', res.status, await res.text());
		return { status: 'failed', firstOffset, tag: tag ?? null, term: term ?? null };
	}

	setHeaders({
		'cache-control': 'public, max-age=60, s-maxage=60'
	});

	const { audiobooks: rows, has_more } = await res.json();

	// Re-root cover URLs so the browser can fetch media straight from the
	// backend origin (or the /api proxy) instead of this SvelteKit server.
	for (const audiobook of rows) {
		audiobook.url = fixClientRoute(audiobook.url);
	}

	return {
		status: 'success',
		firstOffset,
		audiobooks: rows,
		has_more: Boolean(has_more),
		tag: tag ?? null,
		term: term ?? null
	};
}
