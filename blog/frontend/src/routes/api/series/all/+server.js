import { route, fixClientRoute } from '$lib/server/proxy';

export async function GET({ request, fetch }) {
	const authHeader = request.headers.get('Authorization');
	const res = await fetch(route('series/all'), {
		headers: { Authorization: authHeader ?? '' }
	});
	if (!res.ok) return new Response(await res.text(), { status: res.status });
	const data = await res.json();
	data.series?.forEach((s) => {
		s.url = fixClientRoute(s.url);
	});
	return new Response(JSON.stringify(data), {
		status: 200,
		headers: { 'Content-Type': 'application/json' }
	});
}
