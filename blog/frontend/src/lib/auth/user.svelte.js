import { goto } from '$app/navigation';

class AuthStore {
	user = $state();
	isMod = $derived.by(() => {
		const role = this.user?.role;
		return role === 'moderator' || role === 'admin';
	});

	clearLogin() {
		this.user = undefined;
	}
	saveLogin({ username, displayName, token, tokenType, role, avatarUrl }) {
		this.user = { username, displayName, role, token, tokenType, avatarUrl };
	}
	changeDisplayname(displayName) {
		this.user = { ...this.user, displayName };
	}
	changeAvatarUrl(avatarUrl) {
		this.user = { ...this.user, avatarUrl };
	}
	auth() {
		if (!this.user) return '';
		let { token, tokenType } = this.user;
		return `${tokenType} ${token}`;
	}
}

export const authState = new AuthStore();

export function clearLogin() {
	authState.clearLogin();
}
export function saveLogin(data) {
	authState.saveLogin(data);
}
export function changeDisplayname(displayName) {
	authState.changeDisplayname(displayName);
}
export function changeAvatarUrl(avatarUrl) {
	authState.changeAvatarUrl(avatarUrl);
}
export function auth() {
	return authState.auth();
}

export async function login(username, password) {
	let res = await fetch('/api/auth/login', {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({
			username,
			password
		})
	});

	if (!res.ok) {
		authState.clearLogin();
		return {
			status: false,
			message: await res.text()
		};
	}

	const {
		token,
		token_type: tokenType,
		display_name: displayName,
		role,
		avatar_url: avatarUrl
	} = await res.json();

	authState.saveLogin({ username, token, tokenType, displayName, role, avatarUrl });

	return {
		status: true
	};
}

export async function logout() {
	await fetch('/api/auth/logout', { method: 'POST' }).catch(() => {});
	authState.clearLogin();
}

export async function register(username, password, email) {
	const res = await fetch('/api/auth/register', {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({
			username,
			password,
			email
		})
	});

	if (!res.ok) {
		return {
			status: false,
			message: await res.text()
		};
	}

	return {
		status: true,
		success: await res.json()
	};
}

export async function resendVerification(identifier, captchaToken) {
	const res = await fetch('/api/auth/resend-verification', {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({
			identifier,
			captchaToken
		})
	});

	let payload;
	try {
		payload = await res.json();
	} catch {
		payload = { message: await res.text() };
	}

	return {
		status: res.ok,
		message: payload.message ?? 'Request failed.'
	};
}

export async function requestPasswordReset(username, email) {
	const res = await fetch('/api/auth/forgot-password', {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({ username, email })
	});

	let payload;
	try {
		payload = await res.json();
	} catch {
		payload = { message: await res.text() };
	}

	return {
		status: res.ok,
		message: payload.message ?? 'Request failed.'
	};
}

export async function resetPassword(token, password) {
	const res = await fetch('/api/auth/reset-password', {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({ token, password })
	});

	let payload;
	try {
		payload = await res.json();
	} catch {
		payload = { message: await res.text() };
	}

	return {
		status: res.ok,
		message: payload.message ?? 'Request failed.'
	};
}
