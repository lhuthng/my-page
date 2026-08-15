import { decodeShortNames } from '$lib/features/editor/media/references.js';
import { fixClientRoute, route } from '$lib/server/proxy.js';
import { error } from '@sveltejs/kit';

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

	let relatedRes = event.fetch(route(`posts/id/${post_id}/related`), {
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

	data.content = decodeShortNames(data.content, data.medium_short_names);
	data.draft = decodeShortNames(data.draft, data.medium_short_names);

	seriesRes = await seriesRes;
	if (seriesRes.ok) {
		data.series = (await seriesRes.json()).series;

		data.series.forEach((series) => {
			series.url = fixClientRoute(series.url);
		});
	}

	relatedRes = await relatedRes;
	if (relatedRes.ok) {
		data.related_posts = (await relatedRes.json()).posts ?? [];
	} else {
		data.related_posts = [];
	}

	data.cover_url = fixClientRoute(data.cover_url);

	return data;
}
