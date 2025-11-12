import { API_URL } from "$env/static/private";
import { route } from "$lib/server/proxy";

export async function handle({ event, resolve }) {
	const accept = event.request.headers.get("accept") ?? "";
	if (accept.includes("text/html")) {
		const refreshToken = event.cookies.get("refresh-token");

		if (!refreshToken) {
			event.locals.user = null;
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

		res = await event.fetch(route("user/me"), {
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

		const { display_name: displayName, role } = await res.json();

		event.locals.accessToken = { type, token };
		event.locals.user = { displayName, role };
	}
	return resolve(event);
}
