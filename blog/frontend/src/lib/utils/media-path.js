/**
 * Normalize a backend-relative media path into a clean URL segment.
 *
 * The backend stores media URLs as disk-relative paths (`./media/...`), so
 * leading `./` and `.` segments must be stripped before prefixing an origin
 * or `/api` — otherwise URLs like `{origin}/./media/...` only load because
 * browsers happen to normalize dot segments for you.
 *
 * Pure and isomorphic: used by both server code (fixClientRoute) and client
 * components, which is why it lives here and not in $lib/server.
 */
export function normalizeMediaPath(path) {
	return path
		.split('/')
		.filter((segment) => segment !== '' && segment !== '.')
		.join('/');
}

/**
 * Turn a backend media path ("media/i/my-slug") into a URL a browser can
 * fetch, for client-side code that cannot read the server-only BACKEND_ORIGIN
 * (fixClientRoute): everything goes through the same-origin /api proxy.
 * Absolute URLs and already-rooted paths pass through untouched, so the
 * mapping is idempotent.
 */
export function fixUrl(path) {
	if (!path || path.includes('://') || path.startsWith('/')) return path;
	return `/api/${path}`;
}
