import "./internal-CyqLiTQC.js";
import "./exports-BSgHVqs_.js";
import { proxyFallback } from "./proxy-Bm0bd9QW.js";

//#region .svelte-kit/adapter-bun/entries/endpoints/api/users/me/_server.js
async function POST({ request, params, fetch, url }) {
	const res = await proxyFallback({
		request,
		params: { path: `users/me` }
	});
	if (!res.ok) {
		const text = await res.text();
		return new Response(text, { status: res.status });
	}
	return new Response(void 0, { status: 200 });
}

//#endregion
export { POST };
//# sourceMappingURL=_server-BXzJfE15.js.map