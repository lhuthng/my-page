import { fixClientRoute, route } from '$lib/server/proxy.js';
import { codeHighlightPlugin, kaomojiPlugin } from '$lib/custom-rules/index.js';
import { error } from '@sveltejs/kit';
import MarkdownIt from 'markdown-it';

const md = new MarkdownIt().use(codeHighlightPlugin).use(kaomojiPlugin);

export async function load({ fetch, params, setHeaders }) {
	const username = params.slug;

	const [profileRes, commentsRes] = await Promise.all([
		fetch(route(`users/${username}`), { method: 'GET' }),
		fetch(route(`users/${username}/comments?limit=3&offset=0`), {
			method: 'GET'
		})
	]);

	if (profileRes.ok) {
		setHeaders({
			'cache-control': 'public, max-age=60, s-maxage=60'
		});
		const response = await profileRes.json();
		response.avatar_url = fixClientRoute(response.avatar_url);

		const commentsResponse = commentsRes.ok ? await commentsRes.json() : { comments: [] };
		commentsResponse.comments = commentsResponse.comments.map((comment) => ({
			...comment,
			avatar_url: fixClientRoute(comment.avatar_url),
			content_html: md.render(comment.content ?? '')
		}));

		return { response, comments: commentsResponse.comments };
	} else {
		error(404, { message: 'Not found' });
	}
}
