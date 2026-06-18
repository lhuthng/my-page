import { fixClientRoute, route } from '$lib/server/proxy.js';

export async function load({ fetch, setHeaders }) {
	const [postsRes, projectsRes] = await Promise.all([
		fetch(route('posts/featured?limit=5'), {
			method: 'GET',
			headers: { 'Content-Type': 'application/json' }
		}),
		fetch(route('projects/featured?limit=5'), {
			method: 'GET',
			headers: { 'Content-Type': 'application/json' }
		})
	]);

	setHeaders({
		'cache-control': 'public, max-age=10, s-maxage=10'
	});

	let data = {};

	if (postsRes.ok) {
		const postsData = await postsRes.json();
		postsData?.featured_posts?.forEach((post) => {
			if (post.url) post.url = fixClientRoute(post.url);
		});
		data = { ...data, ...postsData };
	} else {
		console.log(await postsRes.text());
	}

	if (projectsRes.ok) {
		const projectsData = await projectsRes.json();
		projectsData?.featured_projects?.forEach((project) => {
			if (project.url) project.url = fixClientRoute(project.url);
		});
		data.featured_projects = projectsData.featured_projects ?? [];
	} else {
		console.log(await projectsRes.text());
	}

	return data;
}
