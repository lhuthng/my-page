import { fixClientRoute, proxyFallback } from '$lib/server/proxy.js';

export async function GET({ request, url }) {
	const res = await proxyFallback({
		request,
		params: { path: 'projects/latest' },
		search: url.search
	});

	if (!res.ok) {
		const text = await res.text();
		return new Response(text, { status: res.status });
	}

	const data = await res.json();

	data.projects?.forEach((project) => {
		if (project.url) project.url = fixClientRoute(project.url);
	});

	return new Response(JSON.stringify(data), { status: 200 });
}
