import { json } from '@sveltejs/kit';
import { route } from '$lib/server/proxy.js';
import { checkAuthLimit, clientHeaders } from '$lib/server/auth-guards.js';

export async function POST({ request, fetch, getClientAddress }) {
	const limited = checkAuthLimit('register', getClientAddress);
	if (limited) return limited;

	let body;
	try {
		body = await request.json();
	} catch {
		return json({ message: 'Invalid request body.' }, { status: 400 });
	}

	if (!body?.username?.trim() || !body?.password || !body?.email?.trim()) {
		return json({ message: 'Username, email, and password are required.' }, { status: 400 });
	}

	const response = await fetch(route('auth/register'), {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
			...clientHeaders(getClientAddress)
		},
		body: JSON.stringify(body)
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
