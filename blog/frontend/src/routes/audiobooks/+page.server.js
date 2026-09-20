import { fixClientRoute, route } from '$lib/server/proxy.js';

export async function load({ fetch, url, setHeaders }) {
	const firstOffset = 12;
	const tag = url.searchParams.get('tag');
	const term = url.searchParams.get('term');

	const params = new URLSearchParams();
	if (tag) params.set('tag', tag);
	if (term) params.set('term', term);
	// Ask for one extra so `has_more` can be derived client-side like
	// /projects (backend has no has_more for audiobooks).
	params.set('limit', String(firstOffset + 1));
	params.set('offset', '0');

	const res = await fetch(route(`audiobooks/public/all?${params}`));

	if (!res.ok) {
		console.error('audiobooks: backend returned', res.status, await res.text());
		return { status: 'failed', firstOffset, tag: tag ?? null, term: term ?? null };
	}

	setHeaders({
		'cache-control': 'public, max-age=60, s-maxage=60'
	});

	const { audiobooks: rows } = await res.json();

	// Re-root cover URLs so the browser can fetch media straight from the
	// backend origin (or the /api proxy) instead of this SvelteKit server.
	for (const audiobook of rows) {
		audiobook.url = fixClientRoute(audiobook.url);
	}

	const has_more = rows.length > firstOffset;

	return {
		status: 'success',
		firstOffset,
		audiobooks: rows.slice(0, firstOffset),
		has_more,
		tag: tag ?? null,
		term: term ?? null
	};
}
