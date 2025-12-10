import { proxyFallback } from "$lib/server/proxy.js";

export async function fallback(event) {
    const { request, params, url } = event;
    return await proxyFallback({ request, params, search: url.search });
}
