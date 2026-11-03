import { fixClientRoute, route } from '$lib/server/proxy.js';
import {
	codeHighlightLazyPlugin,
	loadHighlightJs
} from '$lib/custom-rules/plugins/code-highlight-lazy.js';
import {
	createCommentMarkdown,
	extractMentionUsernames
} from '$lib/features/comments/syntax/markdown.js';
import { error, redirect } from '@sveltejs/kit';

async function buildMentionDictionary(fetch, comments) {
	const dictionary = {};

	for (const comment of comments) {
		if (!comment.username || dictionary[comment.username]) continue;
		dictionary[comment.username] = {
			username: comment.username,
			display_name: comment.display_name,
			avatar_url: fixClientRoute(comment.avatar_url)
		};
	}

	const missing = new Set();
	for (const comment of comments) {
		for (const username of extractMentionUsernames(comment.content)) {
			if (!dictionary[username]) missing.add(username);
		}
	}

	await Promise.all(
		[...missing].map(async (username) => {
			const res = await fetch(
				`${route('users')}?term=${encodeURIComponent(username)}&size=1&offset=0`
			).catch(() => null);
			if (!res?.ok) return;
			const profile = (await res.json().catch(() => null))?.users?.[0];
			if (!profile?.username) return;
			dictionary[username] = {
				username: profile.username,
				display_name: profile.display_name,
				avatar_url: fixClientRoute(profile.avatar_url)
			};
		})
	);

	return dictionary;
}

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
		const mentionDictionary = await buildMentionDictionary(fetch, commentsResponse.comments);
		await loadHighlightJs();
		const md = createCommentMarkdown({
			mentionDictionary,
			highlightPlugin: codeHighlightLazyPlugin
		});
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
