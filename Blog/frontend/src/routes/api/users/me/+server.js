import { fixClientRoute, proxyFallback } from "$lib/server/proxy";

export async function POST({ request, params, fetch, url }) {
    const res = await proxyFallback({
        request,
        params: { path: `users/me` },
    });

    if (!res.ok) {
        const text = await res.text();
        return new Response(text, { status: res.status });
    }

    return new Response(undefined, { status: 200 });
}
