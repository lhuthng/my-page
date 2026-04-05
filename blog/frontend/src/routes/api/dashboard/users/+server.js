import { route, fixClientRoute } from "$lib/server/proxy";

export async function GET({ request, fetch, url }) {
    const authHeader = request.headers.get("Authorization");
    const res = await fetch(route("dashboard/users") + url.search, {
        headers: { Authorization: authHeader ?? "" },
    });
    if (!res.ok) return new Response(await res.text(), { status: res.status });
    const data = await res.json();
    data.users?.forEach(u => { u.avatar_url = fixClientRoute(u.avatar_url); });
    return new Response(JSON.stringify(data), { status: 200, headers: { "Content-Type": "application/json" } });
}
