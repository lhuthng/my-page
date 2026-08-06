import {
	appBlockPlugin,
	codeHighlightPlugin,
	iframeBlockPlugin,
	mediaWithShortcutPlugin,
	namedContainerPlugin,
	revealPlugin,
	slugify,
	youtubeBlockPlugin,
	kaomojiPlugin
} from '$lib/custom-rules/index.js';
import { fixClientRoute, route } from '$lib/server/proxy.js';
import { error } from '@sveltejs/kit';
import MarkdownIt from 'markdown-it';
import mkKatex from 'markdown-it-katex';
import anchor from 'markdown-it-anchor';

export async function load({ fetch, params, setHeaders }) {
	const res = await fetch(route(`projects/s/${params.slug}`), {
		method: 'GET'
	});

	if (!res.ok) {
		throw error(404, 'Project not found');
	}

	setHeaders({
		'cache-control': 'public, max-age=10, s-maxage=10'
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
		rest.v86_runtime.variants = rest.v86_runtime.variants?.map((variant) => ({
			...variant,
			iso_url: fixClientRoute(variant.iso_url)
		}));
	}

	const mediaDictionary = {};
	medium_urls.forEach((url, index) => (mediaDictionary[index.toString()] = fixClientRoute(url)));
	medium_short_names?.forEach((shortName, index) => {
		if (shortName) mediaDictionary[shortName] = fixClientRoute(medium_urls[index]);
	});

	const md = new MarkdownIt()
		.use(mkKatex)
		.use(mediaWithShortcutPlugin, { mediaDictionary })
		.use(iframeBlockPlugin)
		.use(youtubeBlockPlugin)
		.use(appBlockPlugin, { mediaDictionary })
		.use(revealPlugin)
		.use(namedContainerPlugin)
		.use(codeHighlightPlugin)
		.use(kaomojiPlugin)
		.use(anchor, { slugify });
	content = md.render(content);

	return {
		content,
		author_avatar_url,
		cover_url,
		cover_video_url,
		cover_video_type,
		og_image_url,
		demo_url,
		...rest
	};
}
