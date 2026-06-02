import { route } from '$lib/server/proxy';

export async function load({ fetch, setHeaders, url }) {
	const term = url.searchParams.get('term')?.trim() || '';
	const params = new URLSearchParams({ size: '60' });

	if (term) params.set('term', term);

	const res = await fetch(route(`tags?${params.toString()}`), {
		method: 'GET'
	});

	if (res.ok) {
		setHeaders({
			'cache-control': 'public, max-age=60, s-maxage=60'
		});

		const data = await res.json();
		return {
			tags: data.tags ?? [],
			term
		};
	}

	return {
		tags: [],
		term
	};
}
