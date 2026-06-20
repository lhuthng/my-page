import { dateTillNow } from '$lib/utils';
import { fixClientRoute, proxyFallback } from '$lib/server/proxy';
import { getGuestMapping } from '$lib/server/guest-mapping.js';

export async function GET({ request, params, fetch, url }) {
	const res = await proxyFallback({
		request,
		params: { path: `posts/id/${params.id}/comments` },
		search: url.search
	});

	if (!res.ok) {
		const text = await res.text();
		return new Response(text, { status: res.status });
	}

	const data = await res.json();

	data.comments.forEach((comment) => {
		comment.avatar_url = fixClientRoute(comment.avatar_url);
		comment.created_at = dateTillNow(comment.created_at);
		if (!comment.guest_identity) {
			const mapped = getGuestMapping(comment.id);
			if (mapped) comment.guest_identity = mapped;
		}
		if (comment.guest_identity) {
			comment.user_role = null;
			comment.username = null;
			comment.display_name = null;
		}
	});

	return new Response(JSON.stringify(data), { status: 200 });
}
