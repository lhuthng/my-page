import { mediaSyntax } from '$lib/utils';
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

const md = new MarkdownIt()
	.use(mkKatex)
	.use(mediaWithShortcutPlugin)
	.use(iframeBlockPlugin)
	.use(youtubeBlockPlugin)
	.use(appBlockPlugin)
	.use(revealPlugin)
	.use(namedContainerPlugin)
	.use(codeHighlightPlugin)
	.use(kaomojiPlugin)
	.use(anchor, { slugify });

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

		let edits = [...content.matchAll(mediaSyntax)].map((match) => ({
			index: match.index + match[0].lastIndexOf(match[1]),
			length: match[1].length,
			replacement: medium_urls[parseInt(match[1])]
		}));

		edits.sort((a, b) => b.index - a.index);

		author_avatar_url = fixClientRoute(author_avatar_url);
		cover_url = fixClientRoute(cover_url);
		cover_video_url = fixClientRoute(cover_video_url);
		og_image_url = fixClientRoute(og_image_url);

		const mediaDictionary = {};

		medium_urls.forEach((url, index) => (mediaDictionary[index.toString()] = fixClientRoute(url)));
		medium_short_names?.forEach((shortName, index) => {
			if (shortName) mediaDictionary[shortName] = fixClientRoute(medium_urls[index]);
		});

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
