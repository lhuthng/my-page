import { fixClientRoute, route } from '$lib/server/proxy.js';
import { codeHighlightPlugin } from '$lib/custom-rules/plugins/code-highlight.js';
import { kaomojiPlugin } from '$lib/custom-rules';
import { error, redirect } from '@sveltejs/kit';
import MarkdownIt from 'markdown-it';

const md = new MarkdownIt().use(codeHighlightPlugin).use(kaomojiPlugin);

export async function load({ fetch, params, url, locals, setHeaders }) {
	const username = params.slug;

	// The email footer links here with ?section=notifications, but the
	// section only renders for the profile owner. Send everyone else to a
	// page where the section exists instead of a profile missing it.
	if (url.searchParams.get('section') === 'notifications') {
		if (!locals.user) {
			const target = `${url.pathname}${url.search}`;
			throw redirect(303, `/login?redirectTo=${encodeURIComponent(target)}`);
		}
		if (locals.user.username !== username) {
			throw redirect(303, `/profiles/${locals.user.username}?section=notifications`);
		}
	}

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
