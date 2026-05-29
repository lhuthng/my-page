import { fixClientRoute, proxyFallback } from "$lib/server/proxy";

export async function GET({ request, params, url }) {
  const res = await proxyFallback({
    request,
    params: { path: `users/${params.slug}` },
    search: url.search,
  });

  if (!res.ok) {
    const text = await res.text();
    return new Response(text, { status: res.status });
  }

  const user = await res.json();
  user.avatar_url = fixClientRoute(user.avatar_url);

  return new Response(JSON.stringify(user), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}
