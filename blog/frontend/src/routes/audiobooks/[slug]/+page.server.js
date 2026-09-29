import { fixClientRoute, route } from '$lib/server/proxy.js';
import { CHAPTER_WINDOW } from '$lib/players/chapter-windows.js';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params, setHeaders }) {
	// One window of chapters, not the whole book. The player asks for the rest
	// as the reader reaches them — and for the window a saved position sits in —
	// so this page stays cheap however long the book is. `track_count` comes
	// back with the answer and is what the chapter list counts.
	const res = await fetch(
		route(
			`audiobooks/public/s/${encodeURIComponent(params.slug)}` +
				`?tracks_offset=0&tracks_limit=${CHAPTER_WINDOW}`
		)
	);

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
