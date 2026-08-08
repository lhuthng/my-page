import { env } from '$env/dynamic/private';
import { error } from '@sveltejs/kit';

export function route(path) {
	return `${env.API_URL}/${path}`;
}

/**
 * Convert a backend media path (e.g. "media/i/my-slug") into a URL the
 * browser can fetch directly.
 *
 * When BACKEND_ORIGIN is set (e.g. "https://blog-backend.example"),
 * the browser fetches the image straight from the backend, skipping the
 * SvelteKit proxy entirely - one fewer network hop, no memory pressure on
 * the frontend machine, and proper Cache-Control headers reach the browser
 * without being rewritten.
 *
 * When BACKEND_ORIGIN is not set (local dev or not yet configured), it
 * falls back to the existing /api/... proxy route so nothing breaks.
 *
 * NOTE: do NOT prefix this variable with PUBLIC_ - $env/dynamic/private
 * intentionally excludes PUBLIC_-prefixed variables (those belong to
 * $env/dynamic/public). Using a non-prefixed name keeps it server-only
 * while still being readable at runtime.
 */
export function fixClientRoute(path) {
	if (path == undefined) return undefined;
	if (path.includes('://')) return path;
	const backendOrigin = env.BACKEND_ORIGIN;
	if (backendOrigin) {
		return `${backendOrigin.replace(/\/$/, '')}/${path}`;
	}
	return `/api/${path}`.replace('/./', '/');
}

export async function proxyFallback({ request, params, search, extraHeaders }) {
	const url = `${env.API_URL}/${params.path}${search ?? ''}`;
	const headers = { ...Object.fromEntries(request.headers), ...extraHeaders };

	const hasBody = request.method !== 'GET' && request.method !== 'HEAD' && request.body != null;
	if (!hasBody) {
		delete headers['content-length'];
		delete headers['content-type'];
	}
	let body;
	if (hasBody) {
		body = await request.arrayBuffer();
	}

	const proxyRequest = new Request(url, {
		headers,
		method: request.method,
		body,
		...(hasBody && { duplex: 'half' }),
		cache: request.cache,
		credentials: request.credentials,
		integrity: request.integrity,
		keepalive: request.keepalive,
		mode: request.mode,
		redirect: request.redirect,
		referrer: request.referrer,
		referrerPolicy: request.referrerPolicy
	});

	try {
		const response = await fetch(proxyRequest);

		return response;
	} catch (e) {
		console.error('API Proxy Error:', e);
		throw error(503, 'Backend service unavailable.');
	}
}
