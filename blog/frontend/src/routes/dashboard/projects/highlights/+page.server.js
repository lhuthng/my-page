import { fixClientRoute, route } from '$lib/server/proxy.js';
import { error } from '@sveltejs/kit';

export async function load(event) {
	const { role } = await event.parent();
	if (role !== 'admin') {
		throw error(403, 'Unauthorized: Only admins can manage highlights.');
	}

	const res = await event.fetch(route('projects/featured?limit=100'), {
		method: 'GET',
		headers: { 'Content-Type': 'application/json' }
	});

	if (res.ok) {
		const data = await res.json();
		data.featured_projects?.forEach((project) => {
			project.url = fixClientRoute(project.url);
		});
		return {
			featuredProjects: data.featured_projects || []
		};
	} else {
		console.error('Failed to load featured projects:', await res.text());
	}

	return {
		featuredProjects: []
	};
}
