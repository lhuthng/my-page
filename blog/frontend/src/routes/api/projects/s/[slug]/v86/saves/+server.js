import { proxyFallback } from '$lib/server/proxy.js';

export async function GET({ request, params, url }) {
	const res = await proxyFallback({
		request,
		params: { path: `projects/s/${params.slug}/v86/saves` },
		search: url.search
	});
	return new Response(res.body, { status: res.status, headers: res.headers });
}

export async function PUT({ request, params, url }) {
	const res = await proxyFallback({
		request,
		params: { path: `projects/s/${params.slug}/v86/saves` },
		search: url.search
	});
	return new Response(res.body, { status: res.status, headers: res.headers });
}

export async function DELETE({ request, params, url }) {
	const res = await proxyFallback({
		request,
		params: { path: `projects/s/${params.slug}/v86/saves` },
		search: url.search
	});
	return new Response(res.body, { status: res.status, headers: res.headers });
}