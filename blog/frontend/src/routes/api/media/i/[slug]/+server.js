import { fixClientRoute, proxyFallback } from "$lib/server/proxy.js";

export async function GET({ request, params, fetch }) {
  const res = await proxyFallback({
    request,
    params: { path: "media/i/" + params.slug },
  });

  if (!res.ok) {
    const text = await res.text();
    return new Response(text, { status: res.status });
  }

  return new Response(res.body, {
    status: 200,
    headers: {
      "Content-Type":
        res.headers.get("Content-Type") ?? "application/octet-stream",
      "Cache-Control": "public, max-age=31536000, immutable",
    },
  });
}
