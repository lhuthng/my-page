import { __export } from "./chunk-BXaEqquV.js";
import "./internal-CyqLiTQC.js";
import "./exports-BSgHVqs_.js";
import { fixClientRoute, route } from "./proxy-Bm0bd9QW.js";

//#region .svelte-kit/adapter-bun/entries/pages/posts/_page.server.js
var _page_server_exports = {};
__export(_page_server_exports, { load: () => load });
async function load(event) {
	const firstOffset = 10;
	const res = await fetch(route(`posts/latest?limit=${firstOffset}`), { method: "GET" });
	if (res.ok) {
		const data = await res.json();
		data?.featured_posts?.forEach((post) => {
			if (post.url) post.url = fixClientRoute(post.url);
		});
		return {
			status: "success",
			firstOffset,
			...data
		};
	} else {
		console.log(await res.text());
		return { status: "failed" };
	}
}

//#endregion
//#region .svelte-kit/adapter-bun/nodes/16.js
const index = 16;
let component_cache;
const component = async () => component_cache ??= (await import("./_page.svelte-VGLudOb2.js")).default;
const server_id = "src/routes/posts/+page.server.js";
const imports = [
	"_app/immutable/nodes/16.WTRV-dM4.js",
	"_app/immutable/chunks/DsnmJJEf.js",
	"_app/immutable/chunks/ChRVst1r.js",
	"_app/immutable/chunks/DIeogL5L.js",
	"_app/immutable/chunks/keZNrdrU.js",
	"_app/immutable/chunks/DgIQr6Of.js",
	"_app/immutable/chunks/B5M9Mp4x.js",
	"_app/immutable/chunks/CKHu4Vf7.js",
	"_app/immutable/chunks/DSv6HxbV.js",
	"_app/immutable/chunks/BVdBztUd.js",
	"_app/immutable/chunks/su8FdElh.js",
	"_app/immutable/chunks/DfhCrWJZ.js",
	"_app/immutable/chunks/c4lhIocA.js"
];
const stylesheets = ["_app/immutable/assets/PostCard.zaaMa7OM.css"];
const fonts = [];

//#endregion
export { component, fonts, imports, index, _page_server_exports as server, server_id, stylesheets };
//# sourceMappingURL=16-DZ2aQ6Fh.js.map