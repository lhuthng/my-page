import { api } from './client.js';

/**
 * Client for the book-scoped author API.
 *
 * Exposure is per audiobook, not per deployment: `settings` is the switch that
 * makes one book reachable with an `abk_` key, and it is off until the book's
 * creator turns it on.
 *
 * Only the session-side calls live here. The `/author` read/write/download
 * endpoints are driven by the issued secret from a script — that key must never
 * reach a browser session.
 */
export const authorApi = {
	settings: (audiobookId) => api.get(`author/audiobooks/id/${audiobookId}/settings`),

	setEnabled: (audiobookId, enabled) =>
		api.put(`author/audiobooks/id/${audiobookId}/settings`, { body: { enabled } }),

	keys: (audiobookId) => api.get(`author/audiobooks/id/${audiobookId}/keys`),

	issueKey: (audiobookId, payload) =>
		api.post(`author/audiobooks/id/${audiobookId}/keys`, { body: payload }),

	revokeKey: (audiobookId, keyId) => api.delete(`author/audiobooks/id/${audiobookId}/keys/${keyId}`)
};
