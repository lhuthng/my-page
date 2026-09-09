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
			platform_key: system.platform_key,
			base_url: fixClientRoute(system.base_url),
			base_size_bytes: system.size_bytes,
			chunk_size_bytes: system.chunk_size_bytes,
			memory_size_mb: system.memory_size_mb,
			vga_memory_size_mb: system.vga_memory_size_mb,
			screen_width: system.specs?.screen_width ?? null,
			screen_height: system.specs?.screen_height ?? null
		}))
	};
}
