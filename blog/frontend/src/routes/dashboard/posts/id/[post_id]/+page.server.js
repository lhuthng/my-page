import { mediaSyntax } from '$lib/utils';
import { fixClientRoute, route } from '$lib/server/proxy.js';
import { error } from '@sveltejs/kit';

const lottieAppSyntax = /:::app\s+lottie\s+([^\s]+)/g;

function restoreShortNames(text, mediumShortNames) {
	let next = text ?? '';
	const edits = [
		...[...next.matchAll(mediaSyntax)].map((match) => ({
			index: match.index + match[0].lastIndexOf(match[1]),
			length: match[1].length,
			replacement: mediumShortNames[parseInt(match[1])]
		})),
		...[...next.matchAll(lottieAppSyntax)].map((match) => ({
			index: match.index + match[0].lastIndexOf(match[1]),
			length: match[1].length,
			replacement: mediumShortNames[parseInt(match[1])]
		}))
	].filter(({ replacement }) => replacement !== undefined);

	edits.sort((a, b) => b.index - a.index);
	edits.forEach(({ index, length, replacement }) => {
		next = next.slice(0, index) + replacement + next.slice(index + length);
	});
	return next;
}

export async function load(event) {
	const locals = await event.parent();

	const { post_id } = event.params;
	const { type, token } = locals.accessToken;

	let res = event.fetch(route(`posts/id/${post_id}`), {
		method: 'GET',
		headers: { Authorization: `${type} ${token}` }
	});

	let seriesRes = event.fetch(route(`series/all`), {
		method: 'GET',
		headers: { Authorization: `${type} ${token}` }
	});

	res = await res;

	if (!res.ok) {
		console.log(await res.text());
		throw error(404, 'Post Not Found');
	}

	const data = await res.json();

	data.medium_urls = data.medium_urls.map((url) => fixClientRoute(url));
	data.slug_cover_url = fixClientRoute(data.slug_cover_url);

	data.content = restoreShortNames(data.content, data.medium_short_names);
	data.draft = restoreShortNames(data.draft, data.medium_short_names);

	seriesRes = await seriesRes;
	if (seriesRes.ok) {
		data.series = (await seriesRes.json()).series;

		data.series.forEach((series) => {
			series.url = fixClientRoute(series.url);
		});
	}

	data.cover_url = fixClientRoute(data.cover_url);

	return data;
}
