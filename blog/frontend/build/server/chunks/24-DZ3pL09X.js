import { __export } from "./chunk-BXaEqquV.js";
import "./internal-CyqLiTQC.js";
import { redirect } from "./exports-BSgHVqs_.js";

//#region .svelte-kit/adapter-bun/entries/pages/users/_slug_/_page.server.js
var _page_server_exports = {};
__export(_page_server_exports, { load: () => load });
async function load({ params, fetch }) {
	const { slug } = params;
	const res = await fetch(`/api/users/${slug}`, { method: "GET" });
	if (res.ok) return { ...await res.json() };
	else throw redirect(302, "/");
}

//#endregion
//#region .svelte-kit/adapter-bun/nodes/24.js
const index = 24;
let component_cache;
const component = async () => component_cache ??= (await import("./_page.svelte-CeduW9F9.js")).default;
const server_id = "src/routes/users/[slug]/+page.server.js";
const imports = [
	"_app/immutable/nodes/24.DoU81dsD.js",
	"_app/immutable/chunks/DsnmJJEf.js",
	"_app/immutable/chunks/ChRVst1r.js",
	"_app/immutable/chunks/DIeogL5L.js",
	"_app/immutable/chunks/CKHu4Vf7.js",
	"_app/immutable/chunks/keZNrdrU.js",
	"_app/immutable/chunks/DfhCrWJZ.js",
	"_app/immutable/chunks/su8FdElh.js",
	"_app/immutable/chunks/DGag4Teo.js",
	"_app/immutable/chunks/BRm8bsAD.js",
	"_app/immutable/chunks/CscKymCR.js",
	"_app/immutable/chunks/DXBmiNFF.js",
	"_app/immutable/chunks/Dnjdv7Ru.js"
];
const stylesheets = [];
const fonts = [];

//#endregion
export { component, fonts, imports, index, _page_server_exports as server, server_id, stylesheets };
//# sourceMappingURL=24-DZ3pL09X.js.map