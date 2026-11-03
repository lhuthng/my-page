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

export function clientHeaders(getClientAddress) {
	const ip = getClientAddress?.();
	return ip ? { 'x-client-ip': ip } : {};
}

export function checkAuthLimit(scope, getClientAddress) {
	// Local development restarts the server often; a throttle there only gets
	// in the way. The backend limits still apply.
	if (dev) return null;

	const limiter = limiters[scope];
	if (!limiter) return null;

	const result = limiter.check(getClientAddress?.() ?? 'unknown');
	if (result.allowed) return null;

	const retryAfter = Math.ceil(result.resetAfter / 1000);
	return json(
		{ message: `Too many attempts. Try again in ${retryAfter} seconds.` },
		{ status: 429, headers: { 'Retry-After': String(retryAfter) } }
	);
}
