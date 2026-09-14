import { fixClientRoute, route } from '$lib/server/proxy.js';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params, setHeaders }) {
	const res = await fetch(route(`audiobooks/public/s/${encodeURIComponent(params.slug)}`));

	if (res.status === 404) {
		error(404, 'Audiobook not found.');
	}
	if (!res.ok) {
		error(502, 'The audiobook service is unavailable.');
	}

	setHeaders({
		'cache-control': 'public, max-age=60, s-maxage=60'
	});

	const { audiobook } = await res.json();

	audiobook.url = fixClientRoute(audiobook.url);
	// Track URLs become the <audio> element's `src`, so they must be absolute
	// (or proxy-relative) before they reach the browser.
	for (const track of audiobook.tracks) {
		track.url = fixClientRoute(track.url);
	}

	return { audiobook };
}
