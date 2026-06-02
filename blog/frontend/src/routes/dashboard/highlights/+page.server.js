import { fixClientRoute, route } from '$lib/server/proxy';
import { error } from '@sveltejs/kit';

export async function load(event) {
	const { role } = await event.parent();
	if (role !== 'admin') {
		throw error(403, 'Unauthorized: Only admins can manage highlights.');
	}

	// Load currently featured posts
	const res = await event.fetch(route('posts/featured?limit=100'), {
		method: 'GET',
		headers: {
			'Content-Type': 'application/json'
		}
	});

	if (res.ok) {
		const data = await res.json();
		data.featured_posts?.forEach((post) => {
			post.url = fixClientRoute(post.url);
		});
		return {
			featuredPosts: data.featured_posts || []
		};
	} else {
		console.error('Failed to load featured posts:', await res.text());
	}

	return {
		featuredPosts: []
	};
}
