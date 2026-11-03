import { json } from '@sveltejs/kit';
import { route } from '$lib/server/proxy.js';
import { checkAuthLimit, clientHeaders } from '$lib/server/auth-guards.js';

// The Turnstile token is verified by the backend, not here: the API origin is
// reachable directly, so a check in this proxy alone protects only one path
// and a token can only be redeemed once.
export async function POST({ request, fetch, getClientAddress }) {
	const limited = checkAuthLimit('resend', request.headers, getClientAddress);
	if (limited) return limited;

	const { identifier, captchaToken } = await request.json();

	if (!identifier?.trim()) {
		return json({ message: 'Username or email is required.' }, { status: 400 });
	}

	const response = await fetch(route('auth/resend-verification'), {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
			...clientHeaders(request.headers, getClientAddress)
		},
		body: JSON.stringify({ identifier, captchaToken })
	});

	const text = await response.text();
	let payload;
	try {
		payload = JSON.parse(text);
	} catch {
		payload = { message: text };
	}

	return json(payload, { status: response.status });
}
