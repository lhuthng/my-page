import { decodeShortNames } from '$lib/features/editor/media/references.js';
import { fixClientRoute, route } from '$lib/server/proxy.js';
import { error } from '@sveltejs/kit';

export async function load(event) {
	const locals = await event.parent();
	const { project_id } = event.params;
	const { type, token } = locals.accessToken;
	const headers = { Authorization: `${type} ${token}` };

	const res = await event.fetch(route(`projects/id/${project_id}`), {
		method: 'GET',
		headers
	});

	if (!res.ok) {
		console.log(await res.text());
		throw error(404, 'Project not found');
	}

	const data = await res.json();
	data.medium_urls = data.medium_urls.map((url) => fixClientRoute(url));
	data.content = decodeShortNames(data.content, data.medium_short_names);
	data.cover_url = fixClientRoute(data.cover_url);

	const gamesResponse = await event.fetch(route('games/all?limit=100&offset=0'), { headers });
	data.games = gamesResponse.ok ? ((await gamesResponse.json()).games ?? []) : [];

	return data;
}
