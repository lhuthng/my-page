import { error } from '@sveltejs/kit';

/**
 * Only exposes the numeric id.
 *
 * The audiobook itself is fetched in the component with the client API client
 * so a mutation (adding a track, reordering) can refresh the view without a
 * full server round-trip through `invalidate`. The id still has to be validated
 * here so a junk URL fails as a 404 instead of a broken request.
 */
export function load({ params }) {
	const audiobookId = Number(params.audiobook_id);

	if (!Number.isInteger(audiobookId) || audiobookId <= 0) {
		error(404, 'Audiobook not found.');
	}

	return { audiobookId };
}
