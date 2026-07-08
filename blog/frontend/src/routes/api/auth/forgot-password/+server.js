import { json } from '@sveltejs/kit';
import { route } from '$lib/server/proxy.js';

export async function POST({ request, fetch }) {
	const { username, email } = await request.json();

	if (!username?.trim() || !email?.trim()) {
		return json({ message: 'Username and email are required.' }, { status: 400 });
	}

	const response = await fetch(route('auth/forgot-password'), {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({ username, email })
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
