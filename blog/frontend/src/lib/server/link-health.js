import { route } from '$lib/server/proxy.js';

/**
 * Link states for one published body. A failure here is not worth failing the
 * page for: an empty list just means no rewrite and no missing markers.
 */
export async function loadLinkHealth(fetch, slug) {
	try {
		const res = await fetch(route(`links/s/${encodeURIComponent(slug)}`));
		if (!res.ok) return [];
		const data = await res.json();
		return Array.isArray(data.links) ? data.links : [];
	} catch (_) {
		return [];
	}
}
