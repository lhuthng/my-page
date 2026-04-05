import { route, fixClientRoute } from "$lib/server/proxy";

export async function GET({ request, fetch }) {
    const authHeader = request.headers.get("Authorization");
    const res = await fetch(route("dashboard/overview"), {
        headers: { Authorization: authHeader ?? "" },
    });

    if (!res.ok) return new Response(await res.text(), { status: res.status });

    const data = await res.json();

    for (const arr of ["top_posts_by_views", "top_posts_by_likes", "top_posts_by_comments", "recent_posts"]) {
        data[arr]?.forEach(p => { p.url = fixClientRoute(p.url); });
    }

    data.recent_users?.forEach(u => { u.avatar_url = fixClientRoute(u.avatar_url); });

    return new Response(JSON.stringify(data), {
        status: 200,
        headers: { "Content-Type": "application/json" },
    });
}
