import { error } from '@sveltejs/kit';

/** Same id validation as the editor page so a junk URL fails as a 404. */
export function load({ params }) {
	const audiobookId = Number(params.audiobook_id);

	if (!Number.isInteger(audiobookId) || audiobookId <= 0) {
		error(404, 'Audiobook not found.');
	}

	return { audiobookId };
}
