import { route } from "$lib/server/proxy.js";
import { error } from "@sveltejs/kit";

export async function load({ fetch, params }) {
    const username = params.slug;

    const res = await fetch(route(`users/${username}`), {
        method: "GET",
    });

    if (res.ok) {
        return { response: await res.json() };
    } else {
        error(404, { message: "Not found" });
    }
}
