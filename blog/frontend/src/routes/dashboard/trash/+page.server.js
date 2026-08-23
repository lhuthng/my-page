import { route } from '$lib/server/proxy.js';

export async function load(event) {
	const { accessToken } = await event.parent();
	if (!accessToken?.token) return { items: [], error: 'Not authenticated' };
	const { type, token } = accessToken;
	const res = await event.fetch(route('dashboard/trash'), {
		headers: { Authorization: `${type} ${token}` }
	});
	if (!res.ok) {
		return { items: [], error: await res.text() };
	}
	const data = await res.json();
	return { items: data.items ?? [] };
}
