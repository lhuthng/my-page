import { goto } from "$app/navigation";
import { derived, writable } from "svelte/store";

export let user = writable(undefined);

export async function saveLogin({ displayName, token, tokenType }) {
	user.set({
		displayName,
		token,
		tokenType,
	});
}

export async function login(username, password) {
	let res = await fetch("/api/auth/login", {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
		},
		body: JSON.stringify({
			username,
			password,
		}),
	});

	if (!res.ok) {
		user.set(undefined);
		return;
	}

	const {
		token,
		token_type: tokenType,
		display_name: displayName,
	} = await res.json();

	saveLogin({ token, tokenType, displayName });
}

export async function logout() {
	await fetch("/api/auth/logout", { method: "POST" }).catch(() => {});

	user.set(undefined);
}

export async function register(username, password, email) {
	const res = await fetch("/api/auth/register", {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
		},
		body: JSON.stringify({
			username,
			password,
			email,
		}),
	});

	if (!res.ok) {
		return {
			ok: false,
			error: await res.text(),
		};
	}

	return {
		ok: true,
		success: await res.json(),
	};
}
