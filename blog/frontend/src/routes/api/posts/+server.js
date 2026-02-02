import { fixClientRoute, proxyFallback } from "$lib/server/proxy";

export async function GET({ request, params, fetch, url }) {
  const res = await proxyFallback({
    request,
    params: { path: "posts" },
    search: url.search,
  });

  if (!res.ok) {
    const text = await res.text();
    return new Response(text, { status: res.status });
  }

  const { posts } = await res.json();

  posts.forEach((post) => {
    post.cover_url = fixClientRoute(post.cover_url);
  });

  console.log(posts);
  return new Response(JSON.stringify({ posts }), { status: 200 });
}
