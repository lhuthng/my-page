import { route, fixClientRoute } from '$lib/server/proxy.js';
import { error } from '@sveltejs/kit';

// Unauthenticated. The only thing the server is asked for is which system
// images are already publicly fetchable; the ZIP never leaves the browser.
export async function load({ fetch, setHeaders }) {
	const response = await fetch(route('v86/systems/public'));
	if (!response.ok) throw error(response.status, await response.text());

	setHeaders({ 'cache-control': 'no-store' });

	const systems = await response.json();
	// Normalised into exactly the shape SandboxMachine boots from, so the two
	// cannot drift apart on a field name.
	return {
		systems: systems.map((system) => ({
			id: system.id,
			system_name: system.system_name,
			version_number: system.version_number,
			base_url: fixClientRoute(`v86/assets/systems/${system.id}/${system.sha256}/.img.zst`),
			base_size_bytes: system.size_bytes,
			chunk_size_bytes: system.chunk_size_bytes
		}))
	};
}
