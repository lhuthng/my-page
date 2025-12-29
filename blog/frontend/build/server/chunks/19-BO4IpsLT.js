import { __export } from "./chunk-BXaEqquV.js";
import "./internal-CyqLiTQC.js";
import { error } from "./exports-BSgHVqs_.js";
import { fixClientRoute, route } from "./proxy-Bm0bd9QW.js";

//#region .svelte-kit/adapter-bun/entries/pages/profiles/_slug_/_page.server.js
var _page_server_exports = {};
__export(_page_server_exports, { load: () => load });
async function load({ fetch, params }) {
	const username = params.slug;
	const res = await fetch(route(`users/${username}`), { method: "GET" });
	if (res.ok) {
		const response = await res.json();
		response.avatar_url = fixClientRoute(response.avatar_url);
		return { response };
	} else error(404, { message: "Not found" });
}

//#endregion
//#region .svelte-kit/adapter-bun/nodes/19.js
const index = 19;
let component_cache;
const component = async () => component_cache ??= (await import("./_page.svelte-DcYeotim.js")).default;
const server_id = "src/routes/profiles/[slug]/+page.server.js";
const imports = [
	"_app/immutable/nodes/19.j8CxUpIo.js",
	"_app/immutable/chunks/DsnmJJEf.js",
	"_app/immutable/chunks/ChRVst1r.js",
	"_app/immutable/chunks/DIeogL5L.js",
	"_app/immutable/chunks/CKHu4Vf7.js",
	"_app/immutable/chunks/keZNrdrU.js",
	"_app/immutable/chunks/DfhCrWJZ.js",
	"_app/immutable/chunks/su8FdElh.js",
	"_app/immutable/chunks/DgIQr6Of.js",
	"_app/immutable/chunks/DNHieOFC.js",
	"_app/immutable/chunks/c4lhIocA.js",
	"_app/immutable/chunks/C9CqtO5Y.js",
	"_app/immutable/chunks/DGag4Teo.js",
	"_app/immutable/chunks/BRm8bsAD.js",
	"_app/immutable/chunks/B5M9Mp4x.js",
	"_app/immutable/chunks/CQv0etXy.js",
	"_app/immutable/chunks/apr4qJEX.js",
	"_app/immutable/chunks/CscKymCR.js",
	"_app/immutable/chunks/DXBmiNFF.js",
	"_app/immutable/chunks/Dnjdv7Ru.js",
	"_app/immutable/chunks/DSv6HxbV.js",
	"_app/immutable/chunks/BVdBztUd.js",
	"_app/immutable/chunks/BLvaeNRW.js",
	"_app/immutable/chunks/B67vR_oq.js",
	"_app/immutable/chunks/BKQGOQLv.js"
];
const stylesheets = ["_app/immutable/assets/PostCard.zaaMa7OM.css"];
const fonts = [];

//#endregion
export { component, fonts, imports, index, _page_server_exports as server, server_id, stylesheets };
//# sourceMappingURL=19-BO4IpsLT.js.map