import { fixClientRoute, route } from '$lib/server/proxy.js';

export async function load({ fetch, setHeaders }) {
	const firstOffset = 5;
	const res = await fetch(route(`games/latest?limit=${firstOffset}`), {
		method: 'GET'
	});

	if (res.ok) {
		setHeaders({
			'cache-control': 'public, max-age=10, s-maxage=10'
		});
		const data = await res.json();

		data.games?.forEach((game) => {
			if (game.url) game.url = fixClientRoute(game.url);
		});

		return { status: 'success', firstOffset, ...data };
	} else {
		console.log(await res.text());
		return { status: 'failed' };
	}
}
