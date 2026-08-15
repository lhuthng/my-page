import { buildMediaDictionary } from '$lib/features/editor/media/references.js';
import { createMarkdownRenderer } from '$lib/features/editor/markdown/renderer.js';
import { fixClientRoute, route } from '$lib/server/proxy.js';
import { error } from '@sveltejs/kit';

// One module-scoped renderer, reused across requests.
const md = createMarkdownRenderer();

export async function load({ locals, fetch, params, setHeaders }) {
	const res = await fetch(route(`posts/s/${params.slug}`), {
		method: 'GET'
	});

	if (res.ok) {
		setHeaders({
			'cache-control': 'public, max-age=10, s-maxage=60, stale-while-revalidate=300'
		});
		const data = await res.json();

		let {
			content,
			author_avatar_url,
			cover_url,
			cover_video_url,
			cover_video_type,
			og_image_url,
			medium_urls,
			medium_short_names,
			...rest
		} = data;

		author_avatar_url = fixClientRoute(author_avatar_url);
		cover_url = fixClientRoute(cover_url);
		cover_video_url = fixClientRoute(cover_video_url);
		og_image_url = fixClientRoute(og_image_url);

		const mediaDictionary = buildMediaDictionary(medium_urls, medium_short_names, fixClientRoute);

		content = md.render(content, { mediaDictionary });

		const series = rest.series;
		if (series) {
			series.cover_url = fixClientRoute(series.cover_url);
			const fixPost = (post) => {
				if (post) post.cover_url = fixClientRoute(post.cover_url);
			};
			fixPost(series.previous_post);
			fixPost(series.next_post);
		}

		// Fetch related posts (non-fatal - empty array on failure)
		let relatedPosts = [];
		try {
			const relRes = await fetch(route(`posts/id/${rest.id}/related`));
			if (relRes.ok) {
				const relData = await relRes.json();
				relatedPosts = (relData.posts ?? []).map((p) => ({
					...p,
					cover_url: fixClientRoute(p.cover_url)
				}));
			}
		} catch (_) {}

		return {
			content,
			author_avatar_url,
			cover_url,
			cover_video_url,
			cover_video_type,
			og_image_url,
			relatedPosts,
			...rest
		};
	} else {
		console.log(await res.text());
	}

	throw error(404, 'Error');
}
