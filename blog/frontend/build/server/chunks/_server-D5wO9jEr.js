import "./internal-CyqLiTQC.js";
import "./exports-BSgHVqs_.js";
import { fixClientRoute, proxyFallback } from "./proxy-Bm0bd9QW.js";

//#region .svelte-kit/adapter-bun/entries/endpoints/api/media/s/_slug_/_server.js
async function GET({ request, params, fetch }) {
	const res = await proxyFallback({
		request,
		params: { path: "media/s/" + params.slug }
	});
	if (!res.ok) {
		const text = await res.text();
		return new Response(text, { status: res.status });
	}
	const { url } = await res.json();
	return new Response(JSON.stringify({ url: fixClientRoute(url) }), { status: 200 });
}

//#endregion
export { GET };
//# sourceMappingURL=_server-D5wO9jEr.js.map