import { route } from "$lib/server/proxy.js";
import { error } from "@sveltejs/kit";

export async function load(event) {
    const locals = await event.parent();

    const { post_id } = event.params;
    const { type, token } = locals.accessToken;

    const res = await event.fetch(route(`posts/id/${post_id}`), {
        method: "GET",
        headers: { Authorization: `${type} ${token}` },
    });

    if (!res.ok) {
        throw error(404, "Post Not Found");
    }

    return { ...(await res.json()) };
}
