import { json } from '@sveltejs/kit';
import { route } from '$lib/server/proxy.js';

export async function POST({ request, fetch }) {
	const { token, password } = await request.json();

	if (!token?.trim() || !password) {
		return json({ message: 'Reset token and new password are required.' }, { status: 400 });
	}

	const response = await fetch(route('auth/reset-password'), {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({ token, password })
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
