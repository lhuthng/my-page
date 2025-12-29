import { __export } from "./chunk-BXaEqquV.js";
import "./internal-CyqLiTQC.js";
import { error } from "./exports-BSgHVqs_.js";
import { fixClientRoute, route } from "./proxy-Bm0bd9QW.js";
import { mediaSyntax } from "./common-BLrvVRDf.js";

//#region .svelte-kit/adapter-bun/entries/pages/dashboard/posts/id/_post_id_/_page.server.js
var _page_server_exports = {};
__export(_page_server_exports, { load: () => load });
async function load(event) {
	const locals = await event.parent();
	const { post_id } = event.params;
	const { type, token } = locals.accessToken;
	const res = await event.fetch(route(`posts/id/${post_id}`), {
		method: "GET",
		headers: { Authorization: `${type} ${token}` }
	});
	if (!res.ok) throw error(404, "Post Not Found");
	const data = await res.json();
	data.medium_urls = data.medium_urls.map((url) => fixClientRoute(url));
	let { content, draft, medium_short_names } = data;
	let edits = [...content.matchAll(mediaSyntax)].map((match) => ({
		index: match.index + match[0].lastIndexOf(match[1]),
		length: match[1].length,
		replacement: medium_short_names[parseInt(match[1])]
	}));
	edits.sort((a, b) => b.index - a.index);
	edits.forEach(({ index: index$1, length, replacement }) => {
		content = content.slice(0, index$1) + replacement + content.slice(index$1 + length);
	});
	data.content = content;
	edits = [...draft.matchAll(mediaSyntax)].map((match) => ({
		index: match.index + match[0].lastIndexOf(match[1]),
		length: match[1].length,
		replacement: medium_short_names[parseInt(match[1])]
	}));
	edits.sort((a, b) => b.index - a.index);
	edits.forEach(({ index: index$1, length, replacement }) => {
		draft = draft.slice(0, index$1) + replacement + draft.slice(index$1 + length);
	});
	data.draft = draft;
	return { ...data };
}

//#endregion
//#region .svelte-kit/adapter-bun/nodes/12.js
const index = 12;
let component_cache;
const component = async () => component_cache ??= (await import("./_page.svelte-C2X-HJ4J.js")).default;
const server_id = "src/routes/dashboard/posts/id/[post_id]/+page.server.js";
const imports = [
	"_app/immutable/nodes/12.Bs23aKjZ.js",
	"_app/immutable/chunks/DsnmJJEf.js",
	"_app/immutable/chunks/ChRVst1r.js",
	"_app/immutable/chunks/DIeogL5L.js",
	"_app/immutable/chunks/CQ-vwlbD.js",
	"_app/immutable/chunks/CKHu4Vf7.js",
	"_app/immutable/chunks/keZNrdrU.js",
	"_app/immutable/chunks/DfhCrWJZ.js",
	"_app/immutable/chunks/su8FdElh.js",
	"_app/immutable/chunks/c4lhIocA.js",
	"_app/immutable/chunks/B5M9Mp4x.js",
	"_app/immutable/chunks/CQv0etXy.js",
	"_app/immutable/chunks/C9CqtO5Y.js",
	"_app/immutable/chunks/DGag4Teo.js",
	"_app/immutable/chunks/BRm8bsAD.js",
	"_app/immutable/chunks/CscKymCR.js",
	"_app/immutable/chunks/DXBmiNFF.js",
	"_app/immutable/chunks/Dnjdv7Ru.js",
	"_app/immutable/chunks/BKQGOQLv.js",
	"_app/immutable/chunks/CbVeUcFC.js",
	"_app/immutable/chunks/DSv6HxbV.js",
	"_app/immutable/chunks/BVdBztUd.js",
	"_app/immutable/chunks/DgIQr6Of.js",
	"_app/immutable/chunks/C713NCXK.js",
	"_app/immutable/chunks/DlkoDiSb.js",
	"_app/immutable/chunks/Bgs87JQX.js",
	"_app/immutable/chunks/sMxFsjVR.js"
];
const stylesheets = [
	"_app/immutable/assets/PostCard.zaaMa7OM.css",
	"_app/immutable/assets/SearchButton.UdH9qAlD.css",
	"_app/immutable/assets/PostSection.BjzH5kyH.css",
	"_app/immutable/assets/PostEditor.Cel1PQv9.css"
];
const fonts = [];

//#endregion
export { component, fonts, imports, index, _page_server_exports as server, server_id, stylesheets };
//# sourceMappingURL=12-CXua9i7B.js.map