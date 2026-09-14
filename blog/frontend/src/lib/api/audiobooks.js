import { api } from './client.js';

/**
 * Client for the audiobook module.
 *
 * Authoring calls go through the same-origin `/api` proxy with the auth header
 * attached by `client.js`. Audio itself is never fetched here: track `url`
 * values are handed straight to an `<audio>` element so the browser streams
 * them (with HTTP Range requests) instead of buffering a whole file in memory.
 */
export const audiobooks = {
	/** Dashboard listing. Admins see every audiobook; others see their own. */
	list: ({ term, limit = 50, offset = 0 } = {}) => {
		const params = new URLSearchParams();
		if (term) params.set('term', term);
		params.set('limit', String(limit));
		params.set('offset', String(offset));
		return api.get(`audiobooks/all?${params}`);
	},

	/** Full details including ordered tracks, for the editor. */
	details: (audiobookId) => api.get(`audiobooks/id/${audiobookId}`),

	/** The dedicated audiobook tag vocabulary. */
	tags: ({ limit = 100, offset = 0 } = {}) =>
		api.get(`audiobooks/tags?limit=${limit}&offset=${offset}`),

	/**
	 * Create an audiobook. Takes FormData so the optional cover travels with the
	 * metadata; `tags` may be appended more than once.
	 */
	create: (formData) => api.post('audiobooks/new', { body: formData }),

	update: (audiobookId, payload) => api.patch(`audiobooks/id/${audiobookId}`, { body: payload }),

	changeCover: (audiobookId, formData) =>
		api.patch(`audiobooks/id/${audiobookId}/cover`, { body: formData }),

	changeStatus: (audiobookId, status) =>
		api.post(`audiobooks/id/${audiobookId}/status`, { body: { status } }),

	remove: (audiobookId) => api.delete(`audiobooks/id/${audiobookId}`),

	/** Upload one audio track (multipart: file, title, optional number/duration). */
	addTrack: (audiobookId, formData) =>
		api.post(`audiobooks/id/${audiobookId}/tracks`, { body: formData }),

	updateTrack: (audiobookId, trackId, payload) =>
		api.patch(`audiobooks/id/${audiobookId}/tracks/${trackId}`, { body: payload }),

	removeTrack: (audiobookId, trackId) =>
		api.delete(`audiobooks/id/${audiobookId}/tracks/${trackId}`),

	/** `order` must list every track id exactly once. */
	reorderTracks: (audiobookId, order) =>
		api.put(`audiobooks/id/${audiobookId}/tracks/order`, { body: { order } }),

	/** Slug availability; `available: true` means the slug is free. */
	checkSlug: (slug) => api.get(`audiobooks/check?slug=${encodeURIComponent(slug)}`, { auth: false })
};

/**
 * Probe an audio file's duration in the browser.
 *
 * The backend stores whatever the client reports; browsers already have to
 * decode the header to play the file, so this avoids shipping an audio decoder
 * to the server. Resolves to `null` when the metadata cannot be read, which the
 * player tolerates by falling back to the element's own duration.
 */
export function probeAudioDuration(file) {
	return new Promise((resolve) => {
		const url = URL.createObjectURL(file);
		const audio = new Audio();

		const done = (value) => {
			URL.revokeObjectURL(url);
			audio.removeAttribute('src');
			resolve(value);
		};

		// A container the browser cannot decode would otherwise hang forever.
		const timeout = setTimeout(() => done(null), 15000);

		audio.addEventListener('loadedmetadata', () => {
			clearTimeout(timeout);
			const seconds = audio.duration;
			done(Number.isFinite(seconds) && seconds > 0 ? Math.round(seconds) : null);
		});
		audio.addEventListener('error', () => {
			clearTimeout(timeout);
			done(null);
		});

		audio.preload = 'metadata';
		audio.src = url;
	});
}
