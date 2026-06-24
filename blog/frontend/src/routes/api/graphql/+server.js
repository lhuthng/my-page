import { proxyFallback } from '$lib/server/proxy.js';

/** Forward POST GraphQL requests to the backend */
export async function POST(event) {
	const { request, params, url } = event;
	return await proxyFallback({ request, params: { path: 'graphql' }, search: url.search });
}
