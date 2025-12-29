import "./internal-CyqLiTQC.js";
import "./exports-BSgHVqs_.js";
import { fixClientRoute, proxyFallback } from "./proxy-Bm0bd9QW.js";

//#region .svelte-kit/adapter-bun/entries/endpoints/api/users/_server.js
async function GET({ request, params, fetch, url }) {
	const res = await proxyFallback({
		request,
		params: { path: "users" },
		search: url.search
	});
	if (!res.ok) {
		const text = await res.text();
		console.log(text);
		return new Response(text, { status: res.status });
	}
	const { users } = await res.json();
	users.forEach((user) => {
		user.avatar_url = fixClientRoute(user.avatar_url);
	});
	console.log(users);
	return new Response(JSON.stringify({ users }), { status: 200 });
}

//#endregion
export { GET };
//# sourceMappingURL=_server-rQf96XZr.js.map