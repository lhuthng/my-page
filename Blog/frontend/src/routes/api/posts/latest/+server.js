import { fixClientRoute, proxyFallback } from "$lib/server/proxy.js";

export async function GET({ request, fetch, url }) {
    const res = await proxyFallback({
        request,
        params: { path: "posts/latest" },
        search: url.search,
    });

    if (!res.ok) {
        const text = await res.text();
        return new Response(text, { status: res.status });
    }

    const data = await res.json();

    data?.featured_posts?.forEach((post) => {
        if (post.url) post.url = fixClientRoute(post.url);
    });

    return new Response(JSON.stringify(data), { status: 200 });
}
