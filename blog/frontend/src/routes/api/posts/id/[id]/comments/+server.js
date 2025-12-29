import { fixClientRoute, proxyFallback } from "$lib/server/proxy";
import { time } from "$lib/server/time.js";

export async function GET({ request, params, fetch, url }) {
    const res = await proxyFallback({
        request,
        params: { path: `posts/id/${params.id}/comments` },
        search: url.search,
    });

    if (!res.ok) {
        const text = await res.text();
        return new Response(text, { status: res.status });
    }

    const data = await res.json();

    data.comments.forEach((comment) => {
        comment.avatar_url = fixClientRoute(comment.avatar_url);

        const localDate = new Date(comment.created_at.replace(" ", "T"));
        comment.created_at = time.format(localDate, "mini");
    });

    return new Response(JSON.stringify(data), { status: 200 });
}
