import { proxyFallback, route, fix_client_route } from "$lib/server/proxy.js";

export async function GET({ request, fetch, url }) {
    let res = await proxyFallback({
        request,
        params: { path: "media" },
        search: url.search
    });

    if (!res.ok) {
        const text = await res.text();
        return new Response(text, { status: res.status });
    }

    const { results } = await res.json();
    for (let result of results) {
        result.url = fix_client_route(result.url);
    }
    return new Response(
        JSON.stringify({ results }), { status: 200 }
    );
}
