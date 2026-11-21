import { proxyFallback } from '$lib/server/proxy.js';

// One save file of a payload game: read, mirror up, or remove. The body
// streams through, so a save never sits fully resident in the proxy.
export async function GET({ request, params, url }) {
	const res = await proxyFallback({
		request,
		params: { path: `games/s/${params.slug}/saves/${params.key}` },
		search: url.search
	});
	return new Response(res.body, { status: res.status, headers: res.headers });
}

export async function PUT({ request, params, url }) {
	const res = await proxyFallback({
		request,
		params: { path: `games/s/${params.slug}/saves/${params.key}` },
		search: url.search
	});
	return new Response(res.body, { status: res.status, headers: res.headers });
}

export async function DELETE({ request, params, url }) {
	const res = await proxyFallback({
		request,
		params: { path: `games/s/${params.slug}/saves/${params.key}` },
		search: url.search
	});
	return new Response(res.body, { status: res.status, headers: res.headers });
}
