import { fixClientRoute, route } from '$lib/server/proxy.js';

export async function load({ fetch, setHeaders }) {
	const res = await fetch(route('projects/latest?limit=48'), {
		method: 'GET'
	});

	if (!res.ok) {
		return { projects: [] };
	}

	setHeaders({
		'cache-control': 'public, max-age=10, s-maxage=10'
	});

	const { projects } = await res.json();
	return {
		projects: (projects ?? []).map((project) => ({
			...project,
			url: fixClientRoute(project.url)
		}))
	};
}
