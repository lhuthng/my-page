import { canonicalUrl, SITE_ORIGIN } from '$lib/config/site.js';
import { route } from '$lib/server/proxy.js';

const STATIC_PATHS = [
	{ path: '/', priority: 1.0, changefreq: 'daily' },
	{ path: '/posts', priority: 0.6, changefreq: 'daily' },
	{ path: '/projects', priority: 0.6, changefreq: 'weekly' },
	{ path: '/series', priority: 0.5, changefreq: 'monthly' },
	{ path: '/tags', priority: 0.5, changefreq: 'weekly' },
	{ path: '/about', priority: 0.6, changefreq: 'monthly' }
];

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

	const entries = new Map();

	for (const item of STATIC_PATHS) {
		entries.set(item.path, {
			path: item.path,
			priority: item.priority,
			changefreq: item.changefreq
		});
	}

	for (const post of postsData.featured_posts ?? []) {
		if (post.slug) {
			entries.set(`/posts/${encodeURIComponent(post.slug)}`, {
				path: `/posts/${encodeURIComponent(post.slug)}`,
				lastmod: post.updated_at ?? post.published_at,
				priority: 0.8,
				changefreq: 'weekly'
			});
		}
		if (post.author_slug) {
			entries.set(`/profiles/${encodeURIComponent(post.author_slug)}`, {
				path: `/profiles/${encodeURIComponent(post.author_slug)}`,
				priority: 0.6,
				changefreq: 'monthly'
			});
		}
	}

	for (const project of projectsData.projects ?? []) {
		if (project.slug) {
			entries.set(`/projects/${encodeURIComponent(project.slug)}`, {
				path: `/projects/${encodeURIComponent(project.slug)}`,
				lastmod: project.updated_at ?? project.published_at,
				priority: 0.8,
				changefreq: 'monthly'
			});
		}
		if (project.author_slug) {
			entries.set(`/profiles/${encodeURIComponent(project.author_slug)}`, {
				path: `/profiles/${encodeURIComponent(project.author_slug)}`,
				priority: 0.6,
				changefreq: 'monthly'
			});
		}
	}

	for (const tag of tagsData.tags ?? []) {
		if (tag.slug && tag.post_count > 0) {
			entries.set(`/tags/${encodeURIComponent(tag.slug)}`, {
				path: `/tags/${encodeURIComponent(tag.slug)}`,
				priority: 0.4,
				changefreq: 'weekly'
			});
		}
	}

	const urls = [...entries.values()]
		.sort((a, b) => a.path.localeCompare(b.path))
		.map(({ path, lastmod, priority, changefreq }) => {
			const lastmodTag = lastmod ? `\n    <lastmod>${escapeXml(lastmod)}</lastmod>` : '';
			const priorityTag = priority != null ? `\n    <priority>${priority}</priority>` : '';
			const changefreqTag = changefreq ? `\n    <changefreq>${changefreq}</changefreq>` : '';
			return `  <url>\n    <loc>${escapeXml(canonicalUrl(path))}</loc>${lastmodTag}${priorityTag}${changefreqTag}\n  </url>`;
		})
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
