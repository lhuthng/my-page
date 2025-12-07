import { fixClientRoute, proxyFallback } from "$lib/server/proxy";

export async function GET({ request, params, fetch, url }) {
    const res = await proxyFallback({
        request,
        params: { path: `users/${params.slug}/posts` },
        search: url.search,
    });

    if (!res.ok) {
        const text = await res.text();
        return new Response(text, { status: res.status });
    }

    const data = await res.json();

    data?.posts?.forEach((post) => {
        if (post.url) post.url = fixClientRoute(post.url);
    });

    return new Response(JSON.stringify(data), { status: 200 });
}
