import { buildMediaDictionary } from '$lib/features/editor/media/references.js';
import { createMarkdownRenderer } from '$lib/features/editor/markdown/renderer.js';
import { fixClientRoute, route } from '$lib/server/proxy.js';
import { error } from '@sveltejs/kit';

// One module-scoped renderer, reused across requests.
const md = createMarkdownRenderer();

export async function load({ fetch, params, url, setHeaders }) {
	const res = await fetch(route(`projects/s/${params.slug}`), {
		method: 'GET'
	});

	if (!res.ok) {
		throw error(404, 'Project not found');
	}

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
		demo_url,
		...rest
	} = data;

	author_avatar_url = fixClientRoute(author_avatar_url);
	cover_url = fixClientRoute(cover_url);
	cover_video_url = fixClientRoute(cover_video_url);
	og_image_url = fixClientRoute(og_image_url);
	demo_url = fixClientRoute(demo_url);
	if (rest.v86_runtime) {
		rest.v86_runtime.base_url = fixClientRoute(rest.v86_runtime.base_url);
		rest.v86_runtime.iso_url = fixClientRoute(rest.v86_runtime.iso_url);
		rest.v86_runtime.game_url = fixClientRoute(rest.v86_runtime.game_url);
		if (rest.v86_runtime.snapshot_url) {
			rest.v86_runtime.snapshot_url = fixClientRoute(rest.v86_runtime.snapshot_url);
		}
		rest.v86_runtime.variants = rest.v86_runtime.variants?.map((variant) => ({
			...variant,
			iso_url: fixClientRoute(variant.iso_url),
			snapshot_url: variant.snapshot_url ? fixClientRoute(variant.snapshot_url) : undefined
		}));
	}

	const mediaDictionary = buildMediaDictionary(medium_urls, medium_short_names, fixClientRoute);

	content = md.render(content, { mediaDictionary });

	return {
		content,
		author_avatar_url,
		cover_url,
		cover_video_url,
		cover_video_type,
		og_image_url,
		demo_url,
		initialVariant: url.searchParams.get('variant'),
		...rest
	};
}
