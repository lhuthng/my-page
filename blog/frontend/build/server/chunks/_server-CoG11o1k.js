import "./internal-CyqLiTQC.js";
import "./exports-BSgHVqs_.js";
import { proxyFallback } from "./proxy-Bm0bd9QW.js";

//#region .svelte-kit/adapter-bun/entries/endpoints/api/_...path_/_server.js
async function fallback(event) {
	const { request, params, url } = event;
	return await proxyFallback({
		request,
		params,
		search: url.search
	});
}

//#endregion
export { fallback };
//# sourceMappingURL=_server-CoG11o1k.js.map