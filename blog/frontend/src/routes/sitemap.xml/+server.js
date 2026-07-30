import { canonicalUrl, SITE_ORIGIN } from '$lib/config/site.js';
import { route } from '$lib/server/proxy.js';

const STATIC_PATHS = ['/', '/about', '/posts', '/projects', '/series', '/tags'];

function escapeXml(value) {
	return String(value)
		.replaceAll('&', '&amp;')
		.replaceAll('<', '&lt;')
		.replaceAll('>', '&gt;')
		.replaceAll('"', '&quot;')
		.replaceAll("'", '&apos;');
}

async function fetchJson(fetch, path, fallback) {
	try {
		const response = await fetch(route(path));
		return response.ok ? response.json() : fallback;
	} catch {
		return fallback;
	}
}

export async function GET({ fetch }) {
	const [postsData, projectsData, tagsData] = await Promise.all([
		fetchJson(fetch, 'posts/latest?limit=50000&offset=0', { featured_posts: [] }),
		fetchJson(fetch, 'projects/latest?limit=50000&offset=0', { projects: [] }),
		fetchJson(fetch, 'tags?size=50000&offset=0', { tags: [] })
	]);

	const paths = new Set(STATIC_PATHS);

	for (const post of postsData.featured_posts ?? []) {
		if (post.slug) paths.add(`/posts/${encodeURIComponent(post.slug)}`);
		if (post.author_slug) paths.add(`/profiles/${encodeURIComponent(post.author_slug)}`);
	}

	for (const project of projectsData.projects ?? []) {
		if (project.slug) paths.add(`/projects/${encodeURIComponent(project.slug)}`);
		if (project.author_slug) paths.add(`/profiles/${encodeURIComponent(project.author_slug)}`);
	}

	for (const tag of tagsData.tags ?? []) {
		if (tag.slug && tag.post_count > 0) paths.add(`/tags/${encodeURIComponent(tag.slug)}`);
	}

	const urls = [...paths]
		.sort()
		.map((path) => `  <url><loc>${escapeXml(canonicalUrl(path))}</loc></url>`)
		.join('\n');

	const body = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>
`;

	return new Response(body, {
		headers: {
			'Content-Type': 'application/xml; charset=utf-8',
			'Cache-Control': 'public, max-age=300, s-maxage=3600',
			'Content-Location': `${SITE_ORIGIN}/sitemap.xml`
		}
	});
}
