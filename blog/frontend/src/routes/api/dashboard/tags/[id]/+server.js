import { proxyFallback } from '$lib/server/proxy';

export async function PATCH({ request, params }) {
	const res = await proxyFallback({
		request,
		params: { path: `dashboard/tags/${params.id}` }
	});

	const text = await res.text();
	return new Response(text, { status: res.status });
}

export async function DELETE({ request, params }) {
	const res = await proxyFallback({
		request,
		params: { path: `dashboard/tags/${params.id}` }
	});

	if (res.status === 204) {
		return new Response(null, { status: 204 });
	}

	const text = await res.text();
	return new Response(text, { status: res.status });
}
