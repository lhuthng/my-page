import { redirect } from "@sveltejs/kit";

export async function load({ params, fetch }) {
    const { slug } = params;

    const res = await fetch(`/api/users/${slug}`, { method: "GET" });

    if (res.ok) {
        return {
            ...(await res.json()),
        };
    } else {
        throw redirect(302, "/");
    }
}
