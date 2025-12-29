import "./internal-CyqLiTQC.js";
import "./exports-BSgHVqs_.js";
import { fixClientRoute, proxyFallback } from "./proxy-Bm0bd9QW.js";

//#region .svelte-kit/adapter-bun/entries/endpoints/api/posts/s/_slug_/_server.js
async function GET({ request, params, fetch, url }) {
	const res = await proxyFallback({
		request,
		params: { path: `posts/s/${params.slug}` },
		search: url.search
	});
	if (!res.ok) {
		const text = await res.text();
		return new Response(text, { status: res.status });
	}
	const data = await res.json();
	data.medium_urls = data.medium_urls.map((url2) => fixClientRoute(url2));
	return new Response(JSON.stringify(data), { status: 200 });
}

//#endregion
export { GET };
//# sourceMappingURL=_server-do-j6bRl.js.map