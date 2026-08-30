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

// Bodies up to this size are buffered before forwarding; anything larger is
// piped through as a stream so big uploads (v86 disk/ISO chunks) never sit
// fully resident in the frontend process.
const SMALL_BUFFERED_BODY_BYTES = 1024 * 1024;

export async function proxyFallback({ request, params, search, extraHeaders }) {
	const url = `${env.API_URL}/${params.path}${search ?? ''}`;
	const headers = { ...Object.fromEntries(request.headers), ...extraHeaders };

	const hasBody = request.method !== 'GET' && request.method !== 'HEAD' && request.body != null;
	if (!hasBody) {
		delete headers['content-length'];
		delete headers['content-type'];
	}
	const contentLength = Number(request.headers.get('content-length'));
	const streamBody = hasBody && !request.keepalive && contentLength > SMALL_BUFFERED_BODY_BYTES;
	let body;
	if (hasBody) {
		body = streamBody ? request.body : await request.arrayBuffer();
	}

	const proxyRequest = new Request(url, {
		headers,
		method: request.method,
		body,
		...(hasBody && { duplex: 'half' }),
		cache: request.cache,
		credentials: request.credentials,
		integrity: request.integrity,
		// Streaming bodies and keepalive are mutually exclusive in the fetch
		// spec, so keepalive only survives on the buffered path.
		keepalive: !streamBody && request.keepalive,
		mode: request.mode,
		redirect: request.redirect,
		referrer: request.referrer,
		referrerPolicy: request.referrerPolicy
	});

	try {
		const response = await fetch(proxyRequest);

		// The backend runs a CompressionLayer, and we forward the browser's
		// Accept-Encoding, so it may answer with a gzip/br body. fetch already
		// decodes that transparently — but the original Content-Encoding and
		// Content-Length headers survive and would describe the compressed
		// original, leaving the browser to inflate a body that is already
		// plain (ERR_CONTENT_DECODING_FAILED). Drop both so the headers match
		// the bytes we actually forward; compression on the hop to the backend
		// still happens, it just terminates here.
		const responseHeaders = new Headers(response.headers);
		responseHeaders.delete('content-encoding');
		responseHeaders.delete('content-length');

		return new Response(response.body, {
			status: response.status,
			statusText: response.statusText,
			headers: responseHeaders
		});
	} catch (e) {
		console.error('API Proxy Error:', e);
		throw error(503, 'Backend service unavailable.');
	}
}
