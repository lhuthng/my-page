// Access token + profile, cached by the refresh token they were minted from.
//
// Without it, every server-rendered request for a signed-in reader pays two
// backend round-trips (refresh + users/me). Access tokens live 24h, so the
// cached entry is safe to reuse long after the refresh response. The TTL is
// only about picking up profile edits.
//
// Sign-out has to drop the entry: the backend session is revoked by then, and
// serving a cached access token (and profile) for the rest of the TTL would
// outlive the revocation.
const USER_CACHE_TTL_MS = 30 * 60 * 1000;

/** @type {Map<string, { expiresAt: number, result: object }>} */
const cache = new Map();

export function getCachedUser(refreshToken) {
	const entry = cache.get(refreshToken);
	if (!entry) return null;
	if (entry.expiresAt <= Date.now()) {
		cache.delete(refreshToken);
		return null;
	}
	return entry.result;
}

export function cacheUser(refreshToken, result) {
	const now = Date.now();
	for (const [key, entry] of cache) {
		if (entry.expiresAt <= now) cache.delete(key);
	}
	cache.set(refreshToken, { expiresAt: now + USER_CACHE_TTL_MS, result });
}

export function forgetUser(refreshToken) {
	cache.delete(refreshToken);
}
