import { route, fixClientRoute } from "$lib/server/proxy";

export async function GET({ request, fetch, url }) {
    const authHeader = request.headers.get("Authorization");
    const res = await fetch(route("dashboard/posts") + url.search, {
        headers: { Authorization: authHeader ?? "" },
    });
    if (!res.ok) return new Response(await res.text(), { status: res.status });
    const data = await res.json();
    data.posts?.forEach(p => { p.url = fixClientRoute(p.url); });
    return new Response(JSON.stringify(data), { status: 200, headers: { "Content-Type": "application/json" } });
}
