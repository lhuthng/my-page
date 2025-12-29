import "./utils2-B3oc7zZR.js";
import "./internal-CyqLiTQC.js";
import "./exports-BSgHVqs_.js";
import { private_env } from "./internal-CLrXyfXB.js";
import "./clsx-BYt6phfV.js";
import "./escaping-DoGxUxJF.js";
import "./context-BKhPkoFN.js";
import "./chunks-CQcQZdVb.js";
import { fixClientRoute, route } from "./proxy-Bm0bd9QW.js";

//#region .svelte-kit/adapter-bun/chunks/hooks.server.js
async function handle({ event, resolve }) {
	const { API_URL } = private_env;
	console.log("API_URL: ", API_URL);
	const accept = event.request.headers.get("accept") ?? "";
	if (accept.includes("text/html")) {
		const refreshToken = event.cookies.get("refresh-token");
		if (!refreshToken) {
			event.locals.accessToken = null;
			event.locals.user = null;
			event.locals.role = null;
			return await resolve(event);
		}
		let res = await event.fetch(route("auth/refresh"), { method: "POST" });
		if (!res.ok) {
			event.cookies.delete("refresh-token", {
				path: "/",
				httpOnly: true,
				secure: true
			});
			event.locals.user = null;
			return resolve(event);
		}
		const { token_type: type, token } = await res.json();
		res = await event.fetch(route("users/me"), {
			method: "GET",
			headers: {
				Authorization: `${type} ${token}`,
				"Content-Type": "application/json"
			}
		});
		if (!res.ok) {
			event.locals.user = null;
			return resolve(event);
		}
		const { username, display_name: displayName, role, avatar_url: avatarUrl } = await res.json();
		event.locals.accessToken = {
			type,
			token
		};
		event.locals.user = {
			username,
			displayName,
			role,
			avatarUrl: fixClientRoute(avatarUrl)
		};
		event.locals.role = role;
	}
	return resolve(event);
}

//#endregion
export { handle };
//# sourceMappingURL=hooks.server-CUljuapb.js.map