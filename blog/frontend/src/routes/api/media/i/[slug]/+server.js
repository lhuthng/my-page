import { proxyFallback } from "$lib/server/proxy.js";

export async function GET({ request, params }) {
  // Forward conditional request headers so the backend can short-circuit
  // with 304 Not Modified when the browser already has a fresh copy.
  const conditionalHeaders = {};
  const ifNoneMatch = request.headers.get("if-none-match");
  const ifModifiedSince = request.headers.get("if-modified-since");
  if (ifNoneMatch) conditionalHeaders["if-none-match"] = ifNoneMatch;
  if (ifModifiedSince)
    conditionalHeaders["if-modified-since"] = ifModifiedSince;

  const res = await proxyFallback({
    request,
    params: { path: "media/i/" + params.slug },
    extraHeaders: conditionalHeaders,
  });

  // 304 Not Modified — no body, just forward the status so the browser
  // knows its cached copy is still valid.
  if (res.status === 304) {
    return new Response(null, {
      status: 304,
      headers: {
        "Cache-Control": "public, max-age=31536000, immutable",
        ...(res.headers.get("etag") ? { ETag: res.headers.get("etag") } : {}),
      },
    });
  }

  if (!res.ok) {
    const text = await res.text();
    return new Response(text, { status: res.status });
  }

  // Build response headers — stream the body directly (res.body is a
  // ReadableStream) so we never buffer the whole file in the Node process.
  const responseHeaders = {
    "Content-Type":
      res.headers.get("Content-Type") ?? "application/octet-stream",
    "Cache-Control": "public, max-age=31536000, immutable",
  };

  // Forward ETag so browsers can use conditional requests on future reloads
  // (after the 1-year max-age window or when cache is cleared).
  const etag = res.headers.get("etag");
  if (etag) responseHeaders["ETag"] = etag;

  const lastModified = res.headers.get("last-modified");
  if (lastModified) responseHeaders["Last-Modified"] = lastModified;

  return new Response(res.body, {
    status: 200,
    headers: responseHeaders,
  });
}
