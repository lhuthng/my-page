export const prerender = false;
export const ssr = true;

import { clearLogin, saveLogin } from '$lib/auth/user.svelte.js';

export function load({ data }) {
	const { user, accessToken } = data;
	if (user && accessToken) {
		const { username, displayName, role, avatarUrl } = user;
		const { token, type } = accessToken;

		saveLogin({
			username,
			token,
			tokenType: type,
			displayName,
			role,
			avatarUrl
		});
	} else {
		clearLogin();
	}
}
