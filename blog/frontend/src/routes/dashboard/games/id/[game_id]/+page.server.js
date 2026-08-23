import { decodeShortNames } from '$lib/features/editor/media/references.js';
import { fixClientRoute, route } from '$lib/server/proxy.js';
import { error } from '@sveltejs/kit';

export async function load(event) {
	const locals = await event.parent();
	const { game_id } = event.params;
	const { type, token } = locals.accessToken;
	const headers = { Authorization: `${type} ${token}` };

	const res = await event.fetch(route(`games/id/${game_id}`), {
		method: 'GET',
		headers
	});

	if (!res.ok) {
		console.log(await res.text());
		throw error(404, 'Game not found');
	}

	const data = await res.json();
	data.medium_urls = data.medium_urls.map((url) => fixClientRoute(url));
	data.content = decodeShortNames(data.content, data.medium_short_names);
	data.draft = decodeShortNames(data.draft, data.medium_short_names);
	data.cover_url = fixClientRoute(data.cover_url);

	const includeVersion = data.v86_system_version_id
		? `?include_version_id=${data.v86_system_version_id}`
		: '';
	const [systemsResponse, gamesResponse] = await Promise.all([
		event.fetch(route(`v86/systems/active${includeVersion}`), { headers }),
		event.fetch(route('games/all?limit=100&offset=0'), { headers })
	]);
	data.v86Systems = systemsResponse.ok ? await systemsResponse.json() : [];
	data.games = gamesResponse.ok ? ((await gamesResponse.json()).games ?? []) : [];

	return data;
}
