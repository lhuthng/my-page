import { json } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';

const GIPHY_API_KEY = env.GIPHY_API_KEY;

export async function GET({ url }) {
	const query = url.searchParams.get('q') || '';
	const offset = url.searchParams.get('offset') || '0';

	if (!GIPHY_API_KEY) {
		return json({ error: 'GIPHY_API_KEY is not configured on the server.' }, { status: 500 });
	}

	try {
		const endpoint = query
			? `https://api.giphy.com/v1/gifs/search?api_key=${GIPHY_API_KEY}&q=${encodeURIComponent(query)}&limit=18&offset=${offset}&rating=g`
			: `https://api.giphy.com/v1/gifs/trending?api_key=${GIPHY_API_KEY}&limit=18&offset=${offset}&rating=g`;

		const response = await fetch(endpoint);
		if (!response.ok) {
			const errorText = await response.text();
			console.error('Giphy API error details:', errorText);
			return json({ error: 'Failed to fetch from Giphy API' }, { status: response.status });
		}

		const data = await response.json();
		return json(data);
	} catch (error) {
		console.error('Error fetching gifs:', error);
		return json({ error: 'Server error fetching gifs' }, { status: 500 });
	}
}
