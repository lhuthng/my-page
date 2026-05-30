import { fixClientRoute, proxyFallback } from "$lib/server/proxy";

export async function GET({ request, params, fetch }) {
  const res = await proxyFallback({
    request,
    params: { path: `posts/id/${params.id}/related` },
  });

  if (!res.ok) {
    const text = await res.text();
    return new Response(text, { status: res.status });
  }

  const data = await res.json();

  (data.posts ?? []).forEach((post) => {
    post.cover_url = fixClientRoute(post.cover_url);
  });

  return new Response(JSON.stringify(data), { status: 200 });
}

export async function PATCH({ request, params, fetch }) {
  const res = await proxyFallback({
    request,
    params: { path: `posts/id/${params.id}/related` },
  });

  const text = await res.text();
  return new Response(text, { status: res.status });
}
