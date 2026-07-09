import { authState, auth } from '$lib/auth/user.svelte.js';
import { createRequestCache } from './cache.js';
import { createCommentNodeView, normalizeAvatarUrl } from '../model/view.js';

const rootLimit = 3;
const replyLimit = 5;
const commandMaxKaomojis = 12;
const commandMaxGifs = 12;
const drawerKaomojiLimit = 18;

function toCommentsPage(payload) {
	return {
		items: Array.isArray(payload?.comments)
			? payload.comments.map((comment) => createCommentNodeView(comment))
			: [],
		hasMore: Boolean(payload?.has_more)
	};
}

function createCacheSuite() {
	return {
		roots: createRequestCache(),
		replies: createRequestCache(),
		mentions: createRequestCache(),
		commandKaomojis: createRequestCache(),
		commandGifs: createRequestCache(),
		drawerGifs: createRequestCache(),
		drawerKaomojis: createRequestCache()
	};
}

async function fetchCached(cache, key, loader) {
	if (cache.has(key)) {
		return cache.get(key);
	}

	if (cache.inFlight.has(key)) {
		return cache.inFlight.get(key);
	}

	const request = (async () => {
		const value = await loader();
		cache.set(key, value);
		return value;
	})();

	cache.inFlight.set(key, request);
	try {
		return await request;
	} finally {
		cache.inFlight.delete(key);
	}
}

export function createCommentApi() {
	const caches = createCacheSuite();

	return {
		caches,
		clearAll() {
			Object.values(caches).forEach((cache) => cache.clear());
		},
		async fetchRoots(postId, before = null) {
			const key = `${postId}:${before == null ? 'start' : before}`;
			return fetchCached(caches.roots, key, async () => {
				const url =
					before == null
						? `/api/posts/id/${postId}/comments?limit=${rootLimit}`
						: `/api/posts/id/${postId}/comments?limit=${rootLimit}&before=${before}`;
				const res = await fetch(url);
				if (!res.ok) return { items: [], hasMore: false };
				return toCommentsPage(await res.json());
			});
		},
		async fetchReplies(postId, parentId, before = null) {
			const key = `${postId}:${parentId}:${before == null ? 'start' : before}`;
			return fetchCached(caches.replies, key, async () => {
				const url =
					before == null
						? `/api/posts/id/${postId}/comments?parent_id=${parentId}&limit=${replyLimit}`
						: `/api/posts/id/${postId}/comments?parent_id=${parentId}&limit=${replyLimit}&before=${before}`;
				const res = await fetch(url);
				if (!res.ok) return { items: [], hasMore: false };
				return toCommentsPage(await res.json());
			});
		},
		async submitComment({ postId, content, parentId = null, guestIdentity = null }) {
			const currentUser = authState.user;
			const headers =
				currentUser !== undefined
					? { Authorization: auth(), 'Content-Type': 'application/json' }
					: { 'Content-Type': 'application/json' };
			const body = { content, parent_id: parentId };
			if (guestIdentity) body.guest_identity = guestIdentity;

			const res = await fetch(`/api/posts/id/${postId}/comments/new`, {
				method: 'PUT',
				headers,
				body: JSON.stringify(body)
			});

			let payload = null;
			try {
				payload = await res.json();
			} catch {
				payload = null;
			}

			if (!res.ok) {
				return {
					ok: false,
					error: payload?.error || 'Failed to post comment.'
				};
			}

			return {
				ok: true,
				commentId: payload?.comment_id
			};
		},
		async searchMentionProfiles(query, limit = 5) {
			const cacheKey = query.trim().toLowerCase();
			return fetchCached(caches.mentions, cacheKey, async () => {
				const res = await fetch(
					`/api/users?term=${encodeURIComponent(query)}&size=${limit}&offset=0`
				);
				if (!res.ok) return [];
				const data = await res.json();
				return (data.users ?? []).map((profile) => ({
					...profile,
					avatar_url: normalizeAvatarUrl(profile.avatar_url)
				}));
			});
		},
		async searchCommandKaomojis(query) {
			const cacheKey = query.trim().toLowerCase();
			return fetchCached(caches.commandKaomojis, cacheKey, async () => {
				if (!cacheKey) {
					return { items: [], suggestions: [], error: null };
				}

				const res = await fetch(
					`https://kaomoji-search.netlify.app/${encodeURIComponent(cacheKey)}?page=1&limit=${commandMaxKaomojis}`
				);
				const payload = await res.json().catch(() => ({}));

				if (res.status === 404) {
					const suggestions = Array.isArray(payload?.suggestions)
						? payload.suggestions.filter(Boolean)
						: [];
					return {
						items: [],
						suggestions,
						error: suggestions.length === 0 ? 'Mood not found.' : null
					};
				}

				if (!res.ok) {
					return {
						items: [],
						suggestions: [],
						error: 'Could not load kaomojis right now.'
					};
				}

				const items = Array.isArray(payload?.results)
					? payload.results.filter(Boolean).map((value, index) => ({
							id: `kao-${index}-${value}`,
							value
						}))
					: [];

				return {
					items,
					suggestions: [],
					error: items.length === 0 ? 'No kaomojis found for that mood.' : null
				};
			});
		},
		async searchCommandGifs(query) {
			const cacheKey = query.trim().toLowerCase();
			return fetchCached(caches.commandGifs, cacheKey, async () => {
				if (!cacheKey) {
					return { items: [], suggestions: [], error: null };
				}

				const res = await fetch(`/api/gifs?q=${encodeURIComponent(query)}&offset=0`);
				const payload = await res.json().catch(() => ({}));
				if (!res.ok) {
					return {
						items: [],
						suggestions: [],
						error: 'Could not load GIF suggestions.'
					};
				}

				const gifs = Array.isArray(payload?.data) ? payload.data.slice(0, commandMaxGifs) : [];
				const items = gifs.filter(
					(gif) => gif?.images?.original?.url && gif?.images?.fixed_height?.url
				);

				return {
					items,
					suggestions: [],
					error: items.length === 0 ? 'No GIFs found for that query.' : null
				};
			});
		},
		async fetchDrawerGifs(query, offset, reset = false) {
			const cacheKey = `${query.trim().toLowerCase()}|${offset}`;
			return fetchCached(caches.drawerGifs, cacheKey, async () => {
				const res = await fetch(`/api/gifs?q=${encodeURIComponent(query)}&offset=${offset}`);
				const payload = await res.json().catch(() => ({}));
				if (!res.ok) {
					return {
						items: [],
						nextOffset: offset,
						error: 'Could not load GIFs. Please check your connection.'
					};
				}
				const items = Array.isArray(payload?.data) ? payload.data : [];
				return {
					items,
					nextOffset: offset + items.length,
					error: null,
					reset
				};
			});
		},
		async fetchDrawerKaomojis(mood, page) {
			const normalizedMood = mood.trim().toLowerCase();
			const cacheKey = `${normalizedMood}|${page}`;
			return fetchCached(caches.drawerKaomojis, cacheKey, async () => {
				const res = await fetch(
					`https://kaomoji-search.netlify.app/${encodeURIComponent(normalizedMood)}?page=${page}&limit=${drawerKaomojiLimit}`
				);
				const payload = await res.json().catch(() => ({}));

				if (res.status === 404) {
					const suggestions = Array.isArray(payload?.suggestions)
						? payload.suggestions.filter(Boolean)
						: [];
					return {
						items: [],
						suggestions,
						total: 0,
						error: suggestions.length === 0 ? 'Mood not found. Try another search.' : null
					};
				}

				if (!res.ok) {
					return {
						items: [],
						suggestions: [],
						total: 0,
						error: 'Could not load kaomojis right now.'
					};
				}

				const items = Array.isArray(payload?.results) ? payload.results.filter(Boolean) : [];
				return {
					items,
					suggestions: [],
					total: Number(payload?.total ?? 0),
					error: items.length === 0 ? 'No kaomojis found for that mood.' : null
				};
			});
		}
	};
}
