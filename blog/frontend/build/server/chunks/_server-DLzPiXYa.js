import "./internal-CyqLiTQC.js";
import "./exports-BSgHVqs_.js";
import { fixClientRoute, proxyFallback } from "./proxy-Bm0bd9QW.js";
import TimeAgo from "javascript-time-ago";
import en from "javascript-time-ago/locale/en";

//#region .svelte-kit/adapter-bun/entries/endpoints/api/posts/id/_id_/comments/_server.js
TimeAgo.addDefaultLocale(en);
const time = new TimeAgo("en-US");
async function GET({ request, params, fetch, url }) {
	const res = await proxyFallback({
		request,
		params: { path: `posts/id/${params.id}/comments` },
		search: url.search
	});
	if (!res.ok) {
		const text = await res.text();
		return new Response(text, { status: res.status });
	}
	const data = await res.json();
	data.comments.forEach((comment) => {
		comment.avatar_url = fixClientRoute(comment.avatar_url);
		const localDate = new Date(comment.created_at.replace(" ", "T"));
		comment.created_at = time.format(localDate, "mini");
	});
	return new Response(JSON.stringify(data), { status: 200 });
}

//#endregion
export { GET };
//# sourceMappingURL=_server-DLzPiXYa.js.map