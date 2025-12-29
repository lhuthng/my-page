import { __export } from "./chunk-BXaEqquV.js";

//#region .svelte-kit/adapter-bun/entries/pages/login/_page.server.js
var _page_server_exports = {};
__export(_page_server_exports, { load: () => load });
async function load(event) {
	const register = event.url.searchParams.get("register");
	return { register };
}

//#endregion
//#region .svelte-kit/adapter-bun/nodes/15.js
const index = 15;
let component_cache;
const component = async () => component_cache ??= (await import("./_page.svelte-DMM82yhY.js")).default;
const server_id = "src/routes/login/+page.server.js";
const imports = [
	"_app/immutable/nodes/15.BDLB5bIe.js",
	"_app/immutable/chunks/DsnmJJEf.js",
	"_app/immutable/chunks/ChRVst1r.js",
	"_app/immutable/chunks/DIeogL5L.js",
	"_app/immutable/chunks/CKHu4Vf7.js",
	"_app/immutable/chunks/keZNrdrU.js",
	"_app/immutable/chunks/DfhCrWJZ.js",
	"_app/immutable/chunks/su8FdElh.js",
	"_app/immutable/chunks/c4lhIocA.js",
	"_app/immutable/chunks/B5M9Mp4x.js",
	"_app/immutable/chunks/CQv0etXy.js",
	"_app/immutable/chunks/DGag4Teo.js",
	"_app/immutable/chunks/BRm8bsAD.js",
	"_app/immutable/chunks/CscKymCR.js",
	"_app/immutable/chunks/DXBmiNFF.js",
	"_app/immutable/chunks/Dnjdv7Ru.js"
];
const stylesheets = ["_app/immutable/assets/15.DV9fpWxi.css"];
const fonts = [];

//#endregion
export { component, fonts, imports, index, _page_server_exports as server, server_id, stylesheets };
//# sourceMappingURL=15-Beki_0bs.js.map