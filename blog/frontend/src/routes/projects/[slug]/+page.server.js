import { buildMediaDictionary } from '$lib/features/editor/media/references.js';
import { createMarkdownRenderer } from '$lib/features/editor/markdown/renderer.js';
import { loadLinkHealth } from '$lib/server/link-health.js';
import { fixClientRoute, route } from '$lib/server/proxy.js';
import { redirectIfMoved } from '$lib/server/slug-redirect.js';
import { error } from '@sveltejs/kit';

// One module-scoped renderer, reused across requests.
const md = createMarkdownRenderer();

export async function load({ fetch, params, url, setHeaders }) {
	const res = await fetch(route(`projects/s/${params.slug}`), {
		method: 'GET'
	});

	if (!res.ok) {
		await redirectIfMoved(fetch, 'project', params.slug);
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
	if (rest.delegated_game) {
		rest.delegated_game.demo_url = fixClientRoute(rest.delegated_game.demo_url);
		if (rest.delegated_game.jsdos_bundle_url) {
			rest.delegated_game.jsdos_bundle_url = fixClientRoute(rest.delegated_game.jsdos_bundle_url);
		}
		const runtime = rest.delegated_game.v86_runtime;
		if (runtime) {
			runtime.base_url = fixClientRoute(runtime.base_url);
			runtime.iso_url = fixClientRoute(runtime.iso_url);
			runtime.game_url = fixClientRoute(runtime.game_url);
			if (runtime.snapshot_url) {
				runtime.snapshot_url = fixClientRoute(runtime.snapshot_url);
			}
			runtime.variants = runtime.variants?.map((variant) => ({
				...variant,
				iso_url: fixClientRoute(variant.iso_url),
				snapshot_url: variant.snapshot_url ? fixClientRoute(variant.snapshot_url) : undefined
			}));
		}
	}

	const mediaDictionary = buildMediaDictionary(medium_urls, medium_short_names, fixClientRoute);
	const linkHealth = await loadLinkHealth(fetch, params.slug);

	content = md.render(content, { mediaDictionary, linkHealth });

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
