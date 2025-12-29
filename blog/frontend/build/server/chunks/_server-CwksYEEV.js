import "./internal-CyqLiTQC.js";
import "./exports-BSgHVqs_.js";
import { fixClientRoute, proxyFallback } from "./proxy-Bm0bd9QW.js";

//#region .svelte-kit/adapter-bun/entries/endpoints/api/media/_server.js
async function GET({ request, fetch, url }) {
	let res = await proxyFallback({
		request,
		params: { path: "media" },
		search: url.search
	});
	if (!res.ok) {
		const text = await res.text();
		return new Response(text, { status: res.status });
	}
	const { results } = await res.json();
	results.forEach((result) => result.url = fixClientRoute(result.url));
	return new Response(JSON.stringify({ results }), { status: 200 });
}

//#endregion
export { GET };
//# sourceMappingURL=_server-CwksYEEV.js.map