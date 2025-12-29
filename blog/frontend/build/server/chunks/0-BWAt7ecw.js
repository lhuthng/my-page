import { __export } from "./chunk-BXaEqquV.js";
import "./utils2-B3oc7zZR.js";
import "./internal-CyqLiTQC.js";
import "./utils-B7r1aIrD.js";
import "./clsx-BYt6phfV.js";
import "./state.svelte-CeWcm6KF.js";
import { clearLogin, saveLogin } from "./user-CPizBtTY.js";
import { gsap } from "gsap";
import { Flip } from "gsap/Flip.js";
import { ScrollTrigger } from "gsap/ScrollTrigger.js";

//#region .svelte-kit/adapter-bun/entries/pages/_layout.js
var _layout_exports = {};
__export(_layout_exports, {
	load: () => load$1,
	prerender: () => prerender,
	ssr: () => ssr
});
const prerender = false;
const ssr = true;
gsap.registerPlugin(Flip);
gsap.registerPlugin(ScrollTrigger);
function load$1({ data }) {
	const { user, accessToken } = data;
	if (user && accessToken) {
		const { username, displayName, role, avatarUrl } = user;
		const { token, type } = accessToken;
		saveLogin({
			username,
			token,
			tokenType: type,
			displayName,
			role,
			avatarUrl
		});
	} else clearLogin();
}

//#endregion
//#region .svelte-kit/adapter-bun/entries/pages/_layout.server.js
var _layout_server_exports = {};
__export(_layout_server_exports, { load: () => load });
function load(event) {
	const { user, accessToken } = event.locals;
	return {
		user,
		accessToken
	};
}

//#endregion
//#region .svelte-kit/adapter-bun/nodes/0.js
const index = 0;
let component_cache;
const component = async () => component_cache ??= (await import("./_layout.svelte-CsgNshwx.js")).default;
const universal_id = "src/routes/+layout.js";
const server_id = "src/routes/+layout.server.js";
const imports = [
	"_app/immutable/nodes/0.CR0lDoeX.js",
	"_app/immutable/chunks/CscKymCR.js",
	"_app/immutable/chunks/BRm8bsAD.js",
	"_app/immutable/chunks/ChRVst1r.js",
	"_app/immutable/chunks/DIeogL5L.js",
	"_app/immutable/chunks/DXBmiNFF.js",
	"_app/immutable/chunks/Dnjdv7Ru.js",
	"_app/immutable/chunks/DKtf60Sy.js",
	"_app/immutable/chunks/Cy_RWldh.js",
	"_app/immutable/chunks/DsnmJJEf.js",
	"_app/immutable/chunks/keZNrdrU.js",
	"_app/immutable/chunks/DfhCrWJZ.js",
	"_app/immutable/chunks/su8FdElh.js",
	"_app/immutable/chunks/BVdBztUd.js",
	"_app/immutable/chunks/B5M9Mp4x.js",
	"_app/immutable/chunks/CKHu4Vf7.js",
	"_app/immutable/chunks/apr4qJEX.js",
	"_app/immutable/chunks/DGag4Teo.js",
	"_app/immutable/chunks/B5X_EVLB.js",
	"_app/immutable/chunks/69_IOA4Y.js",
	"_app/immutable/chunks/DgIQr6Of.js",
	"_app/immutable/chunks/c4lhIocA.js",
	"_app/immutable/chunks/CQv0etXy.js",
	"_app/immutable/chunks/CbVeUcFC.js",
	"_app/immutable/chunks/DlkoDiSb.js",
	"_app/immutable/chunks/C9CqtO5Y.js",
	"_app/immutable/chunks/BZa1XKl2.js",
	"_app/immutable/chunks/BLvaeNRW.js"
];
const stylesheets = ["_app/immutable/assets/SearchButton.UdH9qAlD.css", "_app/immutable/assets/0.Dtq8m1ji.css"];
const fonts = [];

//#endregion
export { component, fonts, imports, index, _layout_server_exports as server, server_id, stylesheets, _layout_exports as universal, universal_id };
//# sourceMappingURL=0-BWAt7ecw.js.map