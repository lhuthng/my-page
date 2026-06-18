import { route, fixClientRoute } from '$lib/server/proxy';

export async function GET({ request, fetch, params }) {
	const authHeader = request.headers.get('Authorization');
	const res = await fetch(route(`series/id/${params.series_id}/posts`), {
		headers: { Authorization: authHeader ?? '' }
	});
	if (!res.ok) return new Response(await res.text(), { status: res.status });
	const data = await res.json();
	data.posts?.forEach((p) => {
		if (p.cover_url) p.cover_url = fixClientRoute(p.cover_url);
	});
	return new Response(JSON.stringify(data), {
		status: 200,
		headers: { 'Content-Type': 'application/json' }
	});
}
