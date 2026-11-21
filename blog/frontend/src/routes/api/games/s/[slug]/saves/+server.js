import { proxyFallback } from '$lib/server/proxy.js';

// The payload launchers' unified save listing: proxied to the backend's
// /games/s/<slug>/saves the same way the v86 floppies are.
export async function GET({ request, params, url }) {
	const res = await proxyFallback({
		request,
		params: { path: `games/s/${params.slug}/saves` },
		search: url.search
	});
	return new Response(res.body, { status: res.status, headers: res.headers });
}
