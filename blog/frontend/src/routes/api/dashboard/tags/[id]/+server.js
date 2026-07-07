import { proxyFallback } from '$lib/server/proxy';

export async function PATCH({ request, params }) {
	const res = await proxyFallback({
		request,
		params: { path: `dashboard/tags/${params.id}` }
	});

	const text = await res.text();
	return new Response(text, { status: res.status });
}
