import { proxyFallback, route, fixClientRoute } from "$lib/server/proxy.js";

export async function GET({ request, fetch, url }) {
    let res = await proxyFallback({
        request,
        params: { path: "media" },
        search: url.search,
    });

    if (!res.ok) {
        const text = await res.text();
        return new Response(text, { status: res.status });
    }

    const { results } = await res.json();
    results.forEach((result) => (result.url = fixClientRoute(result.url)));
    return new Response(JSON.stringify({ results }), { status: 200 });
}
