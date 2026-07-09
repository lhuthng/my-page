import { env } from '$env/dynamic/private';
import { getGqlClient } from '$lib/api/graphql';
import { OVERVIEW_QUERY } from '$lib/graphql/queries';
import { fixClientRoute, route } from '$lib/server/proxy.js';

function fixOverviewData(overview) {
	if (!overview) return overview;
	for (const key of ['topPostsByViews', 'topPostsByLikes', 'topPostsByComments', 'recentPosts']) {
		if (overview[key]) {
			overview[key] = overview[key].map((p) => ({
				...p,
				coverUrl: fixClientRoute(p.coverUrl)
			}));
		}
	}
	if (overview.recentUsers) {
		overview.recentUsers = overview.recentUsers.map((u) => ({
			...u,
			avatarUrl: fixClientRoute(u.avatarUrl)
		}));
	}
	return overview;
}

export async function load(event) {
	const { accessToken, role } = await event.parent();

	try {
		const client = getGqlClient(event.fetch, env.API_URL, accessToken.token);
		const [result, visitorCountries] = await Promise.all([
			client.request(OVERVIEW_QUERY),
			role === 'admin'
				? event
						.fetch(route('dashboard/analytics/countries?days=30'), {
							headers: { Authorization: `${accessToken.type} ${accessToken.token}` }
						})
						.then((res) => (res.ok ? res.json() : { countries: [] }))
				: Promise.resolve({ countries: [] })
		]);
		return {
			overview: fixOverviewData(result.overview),
			visitorCountries: visitorCountries.countries ?? []
		};
	} catch (e) {
		console.error('[overview] GraphQL error:', e);
		return { overview: null, visitorCountries: [] };
	}
}
