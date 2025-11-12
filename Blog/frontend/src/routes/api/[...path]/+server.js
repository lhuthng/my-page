import { proxyFallback } from "$lib/server/proxy.js";

export async function fallback({ request, params }) {
	return await proxyFallback({ request, params });
}
