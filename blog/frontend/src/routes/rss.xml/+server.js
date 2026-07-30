import { canonicalUrl, SITE_DESCRIPTION, SITE_NAME, SITE_ORIGIN } from '$lib/config/site.js';
import { route } from '$lib/server/proxy.js';

function escapeXml(value) {
	return String(value ?? '')
		.replaceAll('&', '&amp;')
		.replaceAll('<', '&lt;')
		.replaceAll('>', '&gt;')
		.replaceAll('"', '&quot;')
		.replaceAll("'", '&apos;');
}

export async function GET({ fetch }) {
	let data = { featured_posts: [] };
	try {
		const response = await fetch(route('posts/latest?limit=100&offset=0'));
		if (response.ok) data = await response.json();
	} catch {
		// Keep the feed valid while the API is temporarily unavailable.
	}

	const items = (data.featured_posts ?? [])
		.map((post) => {
			const link = canonicalUrl(`/posts/${encodeURIComponent(post.slug)}`);
			return `    <item>
      <title>${escapeXml(post.title)}</title>
      <link>${escapeXml(link)}</link>
      <guid isPermaLink="true">${escapeXml(link)}</guid>
      <description>${escapeXml(post.excerpt)}</description>
      <dc:creator>${escapeXml(post.author_name)}</dc:creator>
    </item>`;
		})
		.join('\n');

	const body = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:dc="http://purl.org/dc/elements/1.1/">
  <channel>
    <title>${escapeXml(SITE_NAME)}</title>
    <link>${escapeXml(`${SITE_ORIGIN}/`)}</link>
    <description>${escapeXml(SITE_DESCRIPTION)}</description>
    <language>en</language>
${items}
  </channel>
</rss>
`;

	return new Response(body, {
		headers: {
			'Content-Type': 'application/rss+xml; charset=utf-8',
			'Cache-Control': 'public, max-age=300, s-maxage=3600'
		}
	});
}
