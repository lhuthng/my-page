import { json } from '@sveltejs/kit';
import { route } from '$lib/server/proxy.js';
import { checkAuthLimit, clientHeaders } from '$lib/server/auth-guards.js';

export async function POST({ request, fetch, getClientAddress }) {
	const limited = checkAuthLimit('forgot', getClientAddress);
	if (limited) return limited;

	const { username, email, captchaToken } = await request.json();

	if (!username?.trim() || !email?.trim()) {
		return json({ message: 'Username and email are required.' }, { status: 400 });
	}

	const response = await fetch(route('auth/forgot-password'), {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
			...clientHeaders(getClientAddress)
		},
		body: JSON.stringify({ username, email, captchaToken })
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
