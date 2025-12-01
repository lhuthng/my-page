import { route } from "$lib/server/proxy.js";

export async function load(event) {
    const res = await fetch(route(`posts/featured?limit=${5}`), {
        method: "GET",
        headers: {
            "Content-Type": "application/json",
        },
    });

    if (res.ok) {
        const data = await res.json();
        return data;
    }
    return {};
}
