import { __export } from "./chunk-BXaEqquV.js";
import "./internal-CyqLiTQC.js";
import { error } from "./exports-BSgHVqs_.js";
import { route } from "./proxy-Bm0bd9QW.js";

//#region .svelte-kit/adapter-bun/entries/pages/dashboard/_layout.server.js
var _layout_server_exports = {};
__export(_layout_server_exports, { load: () => load });
async function load(event) {
	const refreshToken = event.cookies.get("refresh-token");
	if (!refreshToken) throw error(401, "You are not logged in. Redirecting...");
	let { accessToken } = await event.parent();
	if (!accessToken) {
		const res2 = await event.fetch(route("auth/refresh"), { method: "POST" });
		if (!res2.ok) {
			event.cookies.delete("refresh-token", {
				path: "/",
				httpOnly: true,
				secure: true
			});
			throw error(401, "Session expired. Redirecting...");
		}
		const { token_type: type, token } = await res2.json();
		accessToken = {
			type,
			token
		};
	}
	const res = await event.fetch(route("users/me/check-mod"), {
		method: "GET",
		headers: {
			Authorization: `${accessToken.type} ${accessToken.token}`,
			"Content-Type": "application/json"
		}
	});
	if (!res.ok) throw error(403, "Unauthorized: This page is only for moderators and admins. Redirecting...");
	return {
		accessToken,
		...await res.json()
	};
}

//#endregion
//#region .svelte-kit/adapter-bun/nodes/2.js
const index = 2;
let component_cache;
const component = async () => component_cache ??= (await import("./layout.svelte-DnyqhkmQ.js")).default;
const server_id = "src/routes/dashboard/+layout.server.js";
const imports = [
	"_app/immutable/nodes/2.BxC5tyDk.js",
	"_app/immutable/chunks/DsnmJJEf.js",
	"_app/immutable/chunks/ChRVst1r.js",
	"_app/immutable/chunks/DIeogL5L.js",
	"_app/immutable/chunks/keZNrdrU.js",
	"_app/immutable/chunks/BVdBztUd.js",
	"_app/immutable/chunks/su8FdElh.js"
];
const stylesheets = [];
const fonts = [];

//#endregion
export { component, fonts, imports, index, _layout_server_exports as server, server_id, stylesheets };
//# sourceMappingURL=2-CChGm_Uo.js.map