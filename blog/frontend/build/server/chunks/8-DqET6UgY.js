import { __export } from "./chunk-BXaEqquV.js";
import "./internal-CyqLiTQC.js";
import "./exports-BSgHVqs_.js";
import { fixClientRoute, route } from "./proxy-Bm0bd9QW.js";

//#region .svelte-kit/adapter-bun/entries/pages/_page.server.js
var _page_server_exports = {};
__export(_page_server_exports, { load: () => load });
async function load({ fetch }) {
	const res = await fetch(route(`posts/featured?limit=5`), {
		method: "GET",
		headers: { "Content-Type": "application/json" }
	});
	if (res.ok) {
		const data = await res.json();
		data?.featured_posts?.forEach((post) => {
			if (post.url) post.url = fixClientRoute(post.url);
		});
		return data;
	}
	return {};
}

//#endregion
//#region .svelte-kit/adapter-bun/nodes/8.js
const index = 8;
let component_cache;
const component = async () => component_cache ??= (await import("./_page.svelte-DE5aJopw.js")).default;
const server_id = "src/routes/+page.server.js";
const imports = [
	"_app/immutable/nodes/8.Bjjiex1z.js",
	"_app/immutable/chunks/DsnmJJEf.js",
	"_app/immutable/chunks/ChRVst1r.js",
	"_app/immutable/chunks/DIeogL5L.js",
	"_app/immutable/chunks/keZNrdrU.js",
	"_app/immutable/chunks/CKHu4Vf7.js",
	"_app/immutable/chunks/DfhCrWJZ.js",
	"_app/immutable/chunks/su8FdElh.js",
	"_app/immutable/chunks/DgIQr6Of.js",
	"_app/immutable/chunks/c4lhIocA.js",
	"_app/immutable/chunks/B5M9Mp4x.js",
	"_app/immutable/chunks/apr4qJEX.js",
	"_app/immutable/chunks/DSv6HxbV.js",
	"_app/immutable/chunks/BVdBztUd.js",
	"_app/immutable/chunks/Cy_RWldh.js",
	"_app/immutable/chunks/69_IOA4Y.js"
];
const stylesheets = ["_app/immutable/assets/PostCard.zaaMa7OM.css", "_app/immutable/assets/8.DESJIpy5.css"];
const fonts = [];

//#endregion
export { component, fonts, imports, index, _page_server_exports as server, server_id, stylesheets };
//# sourceMappingURL=8-DqET6UgY.js.map