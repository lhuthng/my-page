import { dev } from '$app/environment';
import { json } from '@sveltejs/kit';
import { createRateLimiter } from '$lib/server/rate-limiter.js';

// Edge-level throttle for the auth proxy routes. The backend enforces its own
// per-identifier limits; this one stops a single address from walking through
// many usernames or reset identifiers before the request ever reaches it.
const limiters = {
	login: createRateLimiter({ maxRequests: 10, windowMs: 10 * 60 * 1000 }),
	register: createRateLimiter({ maxRequests: 5, windowMs: 60 * 60 * 1000 }),
	forgot: createRateLimiter({ maxRequests: 5, windowMs: 60 * 60 * 1000 }),
	reset: createRateLimiter({ maxRequests: 10, windowMs: 60 * 60 * 1000 }),
	resend: createRateLimiter({ maxRequests: 5, windowMs: 10 * 60 * 1000 })
};

// The Bun adapter reports the socket peer, which is nginx (127.0.0.1) in
// production. Cloudflare's header is the real client; the forwarded address is
// the fallback for a direct request.
export function clientAddress(headers, getClientAddress) {
	const cloudflare = headers.get('cf-connecting-ip')?.trim();
	if (cloudflare) return cloudflare;

	const forwarded = headers.get('x-forwarded-for')?.split(',')[0]?.trim();
	if (forwarded) return forwarded;

	try {
		return getClientAddress?.() ?? 'unknown';
	} catch {
		return 'unknown';
	}
}

export function clientHeaders(headers, getClientAddress) {
	return { 'x-client-ip': clientAddress(headers, getClientAddress) };
}

export function checkAuthLimit(scope, headers, getClientAddress) {
	// Local development restarts the server often; a throttle there only gets
	// in the way. The backend limits still apply.
	if (dev) return null;

	const limiter = limiters[scope];
	if (!limiter) return null;

	const result = limiter.check(clientAddress(headers, getClientAddress));
	if (result.allowed) return null;

	const retryAfter = Math.ceil(result.resetAfter / 1000);
	return json(
		{ message: `Too many attempts. Try again in ${retryAfter} seconds.` },
		{ status: 429, headers: { 'Retry-After': String(retryAfter) } }
	);
}
