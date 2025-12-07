import { API_URL } from "$env/static/private";
import { route } from "$lib/server/proxy";

export async function handle({ event, resolve }) {
    const accept = event.request.headers.get("accept") ?? "";
    if (accept.includes("text/html")) {
        const refreshToken = event.cookies.get("refresh-token");

        if (!refreshToken) {
            event.locals.accessToken = null;
            event.locals.user = null;
            event.locals.role = null;
            return await resolve(event);
        }

        let res = await event.fetch(route("auth/refresh"), {
            method: "POST",
        });

        if (!res.ok) {
            event.cookies.delete("refresh-token", {
                path: "/",
                httpOnly: true,
                secure: true,
            });
            event.locals.user = null;
            return resolve(event);
        }

        const { token_type: type, token } = await res.json();

        res = await event.fetch(route("users/me"), {
            method: "GET",
            headers: {
                Authorization: `${type} ${token}`,
                "Content-Type": "application/json",
            },
        });

        if (!res.ok) {
            event.locals.user = null;
            return resolve(event);
        }

        const { username, display_name: displayName, role } = await res.json();

        event.locals.accessToken = { type, token };
        event.locals.user = { username, displayName, role };
        event.locals.role = role;
    }

    return resolve(event);
}
