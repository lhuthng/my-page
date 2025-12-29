import { __export } from "./chunk-BXaEqquV.js";
import "./internal-CyqLiTQC.js";
import { error } from "./exports-BSgHVqs_.js";
import { fixClientRoute, route } from "./proxy-Bm0bd9QW.js";
import { mediaSyntax } from "./common-BLrvVRDf.js";
import { mediaWithShortcutPlugin, youtubeBlockPlugin } from "./index2-DUFc1Uun.js";
import MarkdownIt from "markdown-it";

//#region .svelte-kit/adapter-bun/entries/pages/posts/_slug_/_page.server.js
var _page_server_exports = {};
__export(_page_server_exports, { load: () => load });
async function load(event) {
	const res = await fetch(route(`posts/s/${event.params.slug}`), { method: "GET" });
	if (res.ok) {
		const data = await res.json();
		let { content, author_avatar_url, cover_url, medium_urls,...rest } = data;
		let edits = [...content.matchAll(mediaSyntax)].map((match) => ({
			index: match.index + match[0].lastIndexOf(match[1]),
			length: match[1].length,
			replacement: medium_urls[parseInt(match[1])]
		}));
		edits.sort((a, b) => b.index - a.index);
		author_avatar_url = fixClientRoute(author_avatar_url);
		cover_url = fixClientRoute(cover_url);
		const mediaDictionary = {};
		medium_urls.forEach((url, index$1) => mediaDictionary[index$1.toString()] = fixClientRoute(url));
		const md = new MarkdownIt().use(mediaWithShortcutPlugin, { mediaDictionary }).use(youtubeBlockPlugin);
		content = md.render(content);
		return {
			content,
			author_avatar_url,
			cover_url,
			...rest
		};
	} else console.log(await res.text());
	throw error(404, "Error");
}

//#endregion
//#region .svelte-kit/adapter-bun/nodes/17.js
const index = 17;
let component_cache;
const component = async () => component_cache ??= (await import("./_page.svelte-Du_4sJSS.js")).default;
const server_id = "src/routes/posts/[slug]/+page.server.js";
const imports = [
	"_app/immutable/nodes/17.CY_oFLk8.js",
	"_app/immutable/chunks/DsnmJJEf.js",
	"_app/immutable/chunks/ChRVst1r.js",
	"_app/immutable/chunks/DIeogL5L.js",
	"_app/immutable/chunks/keZNrdrU.js",
	"_app/immutable/chunks/BKQGOQLv.js",
	"_app/immutable/chunks/Bgs87JQX.js",
	"_app/immutable/chunks/CKHu4Vf7.js",
	"_app/immutable/chunks/DfhCrWJZ.js",
	"_app/immutable/chunks/su8FdElh.js",
	"_app/immutable/chunks/DgIQr6Of.js",
	"_app/immutable/chunks/c4lhIocA.js",
	"_app/immutable/chunks/DGag4Teo.js",
	"_app/immutable/chunks/BRm8bsAD.js",
	"_app/immutable/chunks/CscKymCR.js",
	"_app/immutable/chunks/DXBmiNFF.js",
	"_app/immutable/chunks/Dnjdv7Ru.js",
	"_app/immutable/chunks/sMxFsjVR.js",
	"_app/immutable/chunks/DNHieOFC.js",
	"_app/immutable/chunks/C9CqtO5Y.js",
	"_app/immutable/chunks/B5M9Mp4x.js",
	"_app/immutable/chunks/CQv0etXy.js",
	"_app/immutable/chunks/apr4qJEX.js",
	"_app/immutable/chunks/DKtf60Sy.js"
];
const stylesheets = ["_app/immutable/assets/PostSection.BjzH5kyH.css"];
const fonts = [];

//#endregion
export { component, fonts, imports, index, _page_server_exports as server, server_id, stylesheets };
//# sourceMappingURL=17-DIo6n7ti.js.map