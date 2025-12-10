import { fixClientRoute, proxyFallback } from "$lib/server/proxy.js";

export async function GET({ request, params, fetch }) {
    const res = await proxyFallback({
        request,
        params: {
            path: "media/s/" + params.slug,
        },
    });

    if (!res.ok) {
        const text = await res.text();
        return new Response(text, { status: res.status });
    }

    const { url } = await res.json();

    return new Response(JSON.stringify({ url: fixClientRoute(url) }), {
        status: 200,
    });
}
