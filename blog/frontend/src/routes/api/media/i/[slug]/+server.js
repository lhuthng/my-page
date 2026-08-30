import { proxyFallback } from '$lib/server/proxy.js';

// Request headers the backend acts on: conditionals short-circuit with 304,
// Range/If-Range produce 206 partial content so <video>/<audio> can seek.
const REQUEST_PASSTHROUGH_HEADERS = ['if-none-match', 'if-modified-since', 'range', 'if-range'];

// Response metadata worth copying alongside the streamed body.
const RESPONSE_PASSTHROUGH_HEADERS = ['etag', 'last-modified', 'content-range'];

export async function GET({ request, params }) {
	const extraHeaders = {};
	for (const name of REQUEST_PASSTHROUGH_HEADERS) {
		const value = request.headers.get(name);
		if (value) extraHeaders[name] = value;
	}

	const res = await proxyFallback({
		request,
		params: { path: 'media/i/' + params.slug },
		extraHeaders
	});

	// 304 Not Modified - no body, just forward the status so the browser
	// knows its cached copy is still valid.
	if (res.status === 304) {
		return new Response(null, {
			status: 304,
			headers: {
				'Cache-Control': 'public, max-age=31536000, immutable',
				...(res.headers.get('etag') ? { ETag: res.headers.get('etag') } : {})
			}
		});
	}

	if (res.status !== 200 && res.status !== 206) {
		const text = await res.text();
		return new Response(text, { status: res.status });
	}

	// Build response headers - stream the body directly (res.body is a
	// ReadableStream) so we never buffer the whole file in the Node process.
	const responseHeaders = {
		'Content-Type': res.headers.get('Content-Type') ?? 'application/octet-stream',
		'Cache-Control': 'public, max-age=31536000, immutable',
		AcceptRanges: 'bytes'
	};

	// Forward ETag so browsers can use conditional requests on future reloads
	// (after the 1-year max-age window or when cache is cleared).
	for (const name of RESPONSE_PASSTHROUGH_HEADERS) {
		const value = res.headers.get(name);
		if (value) responseHeaders[name] = value;
	}

	// The proxy hop strips Content-Length, but partial responses must state
	// their exact body size (Safari refuses range answers without it), so
	// derive it from "bytes start-end/total".
	if (res.status === 206) {
		const match = /^bytes (\d+)-(\d+)\/\d+$/.exec(res.headers.get('content-range') ?? '');
		if (match) {
			responseHeaders['Content-Length'] = String(Number(match[2]) - Number(match[1]) + 1);
		}
	}

	return new Response(res.body, {
		status: res.status,
		headers: responseHeaders
	});
}
