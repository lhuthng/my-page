import { fixClientRoute, route } from '$lib/server/proxy.js';

export async function load({ fetch, setHeaders }) {
	const firstOffset = 10;
	const res = await fetch(route(`projects/latest?limit=${firstOffset}`), {
		method: 'GET'
	});

	if (res.ok) {
		setHeaders({
			'cache-control': 'public, max-age=10, s-maxage=10'
		});
		const data = await res.json();

		data.projects?.forEach((project) => {
			if (project.url) project.url = fixClientRoute(project.url);
		});

		return { status: 'success', firstOffset, ...data };
	} else {
		console.log(await res.text());
		return { status: 'failed' };
	}
}
