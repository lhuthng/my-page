import { route, fixClientRoute } from '$lib/server/proxy.js';
import { error } from '@sveltejs/kit';

export async function load(event) {
	const { accessToken } = await event.parent();
	const { type, token } = accessToken;
	const headers = { Authorization: `${type} ${token}` };
	const gameId = event.params.game_id;

	// The capture runtime resolves by game id (so drafts work) and always
	// comes back without a snapshot: a capture has to start from a cold boot,
	// never from a previously captured state.
	const [runtimeResponse, snapshotResponse] = await Promise.all([
		event.fetch(route(`v86/games/id/${gameId}/capture-runtime`), { headers }),
		event.fetch(route(`v86/games/id/${gameId}/snapshot`), { headers })
	]);

	if (runtimeResponse.status === 404) {
		throw error(404, 'This game has no v86 package attached yet, so there is nothing to capture.');
	}
	if (!runtimeResponse.ok) throw error(runtimeResponse.status, await runtimeResponse.text());
	if (!snapshotResponse.ok) throw error(snapshotResponse.status, await snapshotResponse.text());

	const runtime = await runtimeResponse.json();
	runtime.base_url = fixClientRoute(runtime.base_url);
	runtime.game_url = fixClientRoute(runtime.game_url);
	runtime.iso_url = fixClientRoute(runtime.iso_url);
	runtime.variants = runtime.variants?.map((variant) => ({
		...variant,
		iso_url: fixClientRoute(variant.iso_url)
	}));

	return {
		gameId: Number(gameId),
		runtime,
		snapshots: await snapshotResponse.json()
	};
}
