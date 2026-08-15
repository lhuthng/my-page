import { route } from '$lib/server/proxy.js';

export async function load(event) {
	const { accessToken } = await event.parent();
	const { type, token } = accessToken;
	const headers = { Authorization: `${type} ${token}` };

	try {
		const [subscribersRes, campaignsRes] = await Promise.all([
			event.fetch(route('dashboard/newsletter/subscribers'), { method: 'GET', headers }),
			event.fetch(route('dashboard/newsletter/campaigns'), { method: 'GET', headers })
		]);

		const subscribers = subscribersRes.ok ? (await subscribersRes.json()).subscribers : [];
		const campaigns = campaignsRes.ok ? (await campaignsRes.json()).campaigns : [];

		return { subscribers: subscribers ?? [], campaigns: campaigns ?? [] };
	} catch {
		return { subscribers: [], campaigns: [] };
	}
}
