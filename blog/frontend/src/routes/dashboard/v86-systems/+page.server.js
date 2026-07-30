import { route } from '$lib/server/proxy.js';
import { error } from '@sveltejs/kit';

export async function load(event) {
	const { accessToken } = await event.parent();
	const { type, token } = accessToken;
	const response = await event.fetch(route('v86/systems'), {
		headers: { Authorization: `${type} ${token}` }
	});
	if (!response.ok) throw error(response.status, await response.text());
	return { systems: await response.json() };
}
