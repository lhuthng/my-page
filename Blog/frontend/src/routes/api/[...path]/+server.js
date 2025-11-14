import { proxyFallback } from "$lib/server/proxy.js";

export async function fallback({ request, params }) {
    console.log("proxied");
    return await proxyFallback({ request, params });
}
