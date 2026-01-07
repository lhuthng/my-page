import { fixClientRoute, proxyFallback } from "$lib/server/proxy.js";

export async function GET({ request, params, fetch, url }) {
  const res = await proxyFallback({
    request,
    params: { path: `posts/s/${params.slug}` },
    search: url.search,
  });

  if (!res.ok) {
    const text = await res.text();
    return new Response(text, { status: res.status });
  }

  const data = await res.json();

  data.medium_urls = data.medium_urls.map((url) => fixClientRoute(url));

  return new Response(JSON.stringify(data), { status: 200 });
}
