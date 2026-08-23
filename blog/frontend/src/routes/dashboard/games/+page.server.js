import { fixClientRoute, route } from '$lib/server/proxy.js';

export async function load(event) {
	const { accessToken } = await event.parent();
	const { type, token } = accessToken;
	const response = await event.fetch(route('games/all?limit=9&offset=0'), {
		headers: { Authorization: `${type} ${token}` }
	});
	if (!response.ok) {
		return { games: [], hasMore: false };
	}
	const data = await response.json();
	data.games?.forEach((game) => {
		if (game.url) game.url = fixClientRoute(game.url);
	});
	return { games: data.games ?? [], hasMore: Boolean(data.has_more) };
}
