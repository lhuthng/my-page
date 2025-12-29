import { error } from "./exports-BSgHVqs_.js";

//#region .svelte-kit/adapter-bun/chunks/proxy.js
const API_URL = "http://localhost:3000";
function route(path) {
	return `${API_URL}/${path}`;
}
function fixClientRoute(path) {
	if (path === void 0) return void 0;
	return `/api/${path}`.replace("/./", "/");
}
async function proxyFallback({ request, params, search }) {
	const url = `${API_URL}/${params.path}${search ?? ""}`;
	const proxyRequest = new Request(url, {
		headers: request.headers,
		method: request.method,
		body: request.body,
		duplex: "half",
		cache: request.cache,
		credentials: request.credentials,
		integrity: request.integrity,
		keepalive: request.keepalive,
		mode: request.mode,
		redirect: request.redirect,
		referrer: request.referrer,
		referrerPolicy: request.referrerPolicy
	});
	try {
		const response = await fetch(proxyRequest);
		return response;
	} catch (e) {
		console.error("API Proxy Error:", e);
		throw error(503, "Backend service unavailable.");
	}
}

//#endregion
export { fixClientRoute, proxyFallback, route };
//# sourceMappingURL=proxy-Bm0bd9QW.js.map