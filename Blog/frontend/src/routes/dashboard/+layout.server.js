import { route } from "$lib/server/proxy";
import { redirect, error } from "@sveltejs/kit";

export async function load(event) {
    const refreshToken = event.cookies.get("refresh-token");

    if (!refreshToken) {
        throw error(401, "You are not logged in. Redirecting...");
    }

    let { accessToken } = await event.parent();

    if (!accessToken) {
        const res = await event.fetch(route("auth/refresh"), {
            method: "POST",
        });

        if (!res.ok) {
            event.cookies.delete("refresh-token", {
                path: "/",
                httpOnly: true,
                secure: true,
            });
            throw error(401, "Session expired. Redirecting...");
        }
        const { token_type: type, token } = await res.json();
        accessToken = { type, token };
    }

    const res = await event.fetch(route("users/me/check-mod"), {
        method: "GET",
        headers: {
            Authorization: `${accessToken.type} ${accessToken.token}`,
            "Content-Type": "application/json",
        },
    });

    if (!res.ok) {
        throw error(
            403,
            "Unauthorized: This page is only for moderators and admins. Redirecting...",
        );
    }

    return {
        accessToken,
        ...(await res.json()),
    };
}
