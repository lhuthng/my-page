import { dev } from '$app/environment';
import { env } from '$env/dynamic/private';
import { json } from '@sveltejs/kit';
import { route } from '$lib/server/proxy.js';

async function verifyTurnstile({ captchaToken, remoteIp, fetch }) {
	if (!env.TURNSTILE_SECRET_KEY) {
		return {
			ok: false,
			status: 500,
			message: 'Captcha is not configured on the server.'
		};
	}

	const form = new URLSearchParams({
		secret: env.TURNSTILE_SECRET_KEY,
		response: captchaToken
	});

	if (remoteIp) {
		form.set('remoteip', remoteIp);
	}

	const response = await fetch('https://challenges.cloudflare.com/turnstile/v0/siteverify', {
		method: 'POST',
		headers: {
			'Content-Type': 'application/x-www-form-urlencoded'
		},
		body: form.toString()
	});

	if (!response.ok) {
		return {
			ok: false,
			status: 502,
			message: 'Captcha verification failed.'
		};
	}

	const payload = await response.json();
	if (!payload.success) {
		return {
			ok: false,
			status: 400,
			message: 'Captcha verification failed.'
		};
	}

	return { ok: true };
}

export async function POST({ request, fetch, getClientAddress }) {
	const { email, turnstileToken } = await request.json();

	if (!email?.trim()) {
		return json({ message: 'Email is required.' }, { status: 400 });
	}

	if (!dev && !turnstileToken) {
		return json({ message: 'Captcha is required.' }, { status: 400 });
	}

	if (!dev) {
		const verification = await verifyTurnstile({
			captchaToken: turnstileToken,
			remoteIp: getClientAddress?.(),
			fetch
		});

		if (!verification.ok) {
			return json({ message: verification.message }, { status: verification.status });
		}
	}

	const response = await fetch(route('newsletter/subscribe'), {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({ email: email.trim() })
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
