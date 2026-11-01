import { previewParts } from '$lib/server/notification-preview';
import { proxyFallback } from '$lib/server/proxy';

export async function GET({ request, url }) {
	const res = await proxyFallback({
		request,
		params: { path: 'notifications' },
		search: url.search
	});

	if (!res.ok) {
		const text = await res.text();
		return new Response(text, { status: res.status });
	}

	const { items, has_more } = await res.json();

	return new Response(
		JSON.stringify({
			has_more,
			items: items.map((item) => ({
				...item,
				comment: { id: item.comment.id, preview: previewParts(item.comment.content) }
			}))
		}),
		{ status: 200, headers: { 'content-type': 'application/json' } }
	);
}
