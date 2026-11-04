import { redirect } from '@sveltejs/kit';
import { route } from '$lib/server/proxy.js';

const PATH_BY_KIND = {
	post: 'posts',
	project: 'projects',
	game: 'games',
	series: 'series'
};

/**
 * Called from a page loader right before its 404. If the backend remembers
 * that this entity moved away from `slug`, send a permanent redirect to the
 * current URL instead of a dead page.
 */
export async function redirectIfMoved(fetch, kind, slug) {
	let target = null;
	try {
		const res = await fetch(route(`redirects/${kind}/${encodeURIComponent(slug)}`));
		if (res.ok) target = await res.json();
	} catch (_) {}

	if (!target?.slug || target.slug === slug) return;

	const prefix = PATH_BY_KIND[target.kind] ?? 'posts';
	throw redirect(301, `/${prefix}/${encodeURIComponent(target.slug)}`);
}
