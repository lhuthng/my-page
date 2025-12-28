import { fixClientRoute, proxyFallback } from "$lib/server/proxy";

export async function GET({ request, params, fetch, url }) {
    const res = await proxyFallback({
        request,
        params: { path: "users" },
        search: url.search,
    });

    if (!res.ok) {
        const text = await res.text();
        console.log(text);
        return new Response(text, { status: res.status });
    }

    const { users } = await res.json();

    users.forEach((user) => {
        user.avatar_url = fixClientRoute(user.avatar_url);
    });

    console.log(users);
    return new Response(JSON.stringify({ users }), { status: 200 });
}
