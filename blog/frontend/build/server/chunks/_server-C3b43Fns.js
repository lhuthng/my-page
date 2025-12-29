import "./internal-CyqLiTQC.js";
import "./exports-BSgHVqs_.js";
import { fixClientRoute, proxyFallback } from "./proxy-Bm0bd9QW.js";

//#region .svelte-kit/adapter-bun/entries/endpoints/api/posts/_server.js
async function GET({ request, params, fetch, url }) {
	const res = await proxyFallback({
		request,
		params: { path: "posts" },
		search: url.search
	});
	if (!res.ok) {
		const text = await res.text();
		console.log(text);
		return new Response(text, { status: res.status });
	}
	const { posts } = await res.json();
	posts.forEach((post) => {
		post.cover_url = fixClientRoute(post.cover_url);
	});
	console.log(posts);
	return new Response(JSON.stringify({ posts }), { status: 200 });
}

//#endregion
export { GET };
//# sourceMappingURL=_server-C3b43Fns.js.map