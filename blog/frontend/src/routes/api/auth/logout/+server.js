import { forgetUser } from '$lib/server/user-cache.js';
import { route } from '$lib/server/proxy.js';

export async function POST({ cookies, fetch }) {
	const refreshToken = cookies.get('refresh-token');

	if (refreshToken) {
		// End the session server-side before dropping the cookie. Without this
		// the refresh token stays valid until it expires, so a copy taken before
		// sign-out keeps minting access tokens.
		await fetch(route('auth/logout'), {
			method: 'POST',
			headers: { cookie: `refresh-token=${refreshToken}` }
		}).catch(() => {});

		forgetUser(refreshToken);
	}

	cookies.delete('refresh-token', {
		path: '/',
		httpOnly: true,
		secure: true,
		sameSite: 'lax'
	});
	return new Response(null, { status: 204 });
}
