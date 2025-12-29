import "./utils2-B3oc7zZR.js";
import "./internal-CyqLiTQC.js";
import "./exports-BSgHVqs_.js";
import "./utils-B7r1aIrD.js";
import "./clsx-BYt6phfV.js";
import { fixClientRoute, proxyFallback, route } from "./proxy-Bm0bd9QW.js";
import "./state.svelte-CeWcm6KF.js";
import "./user-CPizBtTY.js";

//#region .svelte-kit/adapter-bun/entries/endpoints/api/auth/login/_server.js
async function POST({ request, fetch }) {
	let res = await proxyFallback({
		request,
		params: { path: "auth/login" }
	});
	if (!res.ok) {
		const text = await res.text();
		return new Response(text, { status: res.status });
	}
	const setCookieHeader = res.headers.get("Set-Cookie");
	const { token, token_type } = await res.json();
	res = await fetch(route("users/me"), {
		method: "GET",
		headers: {
			Authorization: `${token_type} ${token}`,
			"Content-Type": "application/json"
		}
	});
	if (!res.ok) {
		const text = await res.text();
		return new Response(text, { status: res.status });
	}
	const { username, display_name, role, avatar_url } = await res.json();
	const headers = new Headers({ "Content-Type": "application/json" });
	headers.set("Set-Cookie", setCookieHeader);
	return new Response(JSON.stringify({
		username,
		display_name,
		role,
		token,
		token_type,
		avatar_url: fixClientRoute(avatar_url)
	}), {
		status: 200,
		headers
	});
}

//#endregion
export { POST };
//# sourceMappingURL=_server-B7AI8Pwn.js.map