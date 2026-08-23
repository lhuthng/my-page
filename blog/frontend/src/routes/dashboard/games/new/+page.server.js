import { route } from '$lib/server/proxy.js';

export async function load(event) {
	const { accessToken } = await event.parent();
	const { type, token } = accessToken;
	const response = await event.fetch(route('v86/systems/active'), {
		headers: { Authorization: `${type} ${token}` }
	});
	// The related-games picker lists the author's own games.
	const gamesResponse = await event.fetch(route('games/all?limit=100&offset=0'), {
		headers: { Authorization: `${type} ${token}` }
	});
	return {
		v86Systems: response.ok ? await response.json() : [],
		games: gamesResponse.ok ? ((await gamesResponse.json()).games ?? []) : []
	};
}
