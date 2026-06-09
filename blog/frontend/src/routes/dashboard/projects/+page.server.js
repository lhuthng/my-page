import { fixClientRoute, route } from '$lib/server/proxy.js';

export async function load(event) {
	const { accessToken } = await event.parent();
	const { type, token } = accessToken;

	const res = await event.fetch(route('projects/all?limit=100'), {
		method: 'GET',
		headers: { Authorization: `${type} ${token}` }
	});

	if (!res.ok) return { projects: [] };

	const { projects } = await res.json();
	return {
		projects: (projects ?? []).map((project) => ({
			...project,
			url: fixClientRoute(project.url)
		}))
	};
}
