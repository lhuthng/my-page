import { goto } from "$app/navigation";
import { derived, get, writable } from "svelte/store";

export let user = writable(undefined);
export let isMod = derived(user, ($user) => {
    const role = $user?.role;
    return role === "moderator" || role === "admin";
});

export function clearLogin() {
    user.set(undefined);
}
export function saveLogin({
    username,
    displayName,
    token,
    tokenType,
    role,
    avatarUrl,
}) {
    user.set({
        username,
        displayName,
        role,
        token,
        tokenType,
        avatarUrl,
    });
}

export function changeDisplayname(displayName) {
    user.update(($user) => ({ ...$user, displayName }));
}

export function auth() {
    let { token, tokenType } = get(user);
    return `${tokenType} ${token}`;
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
        return {
            status: false,
            message: await res.text(),
        };
    }

    const {
        token,
        token_type: tokenType,
        display_name: displayName,
        role,
        avatar_url: avatarUrl,
    } = await res.json();

    saveLogin({ username, token, tokenType, displayName, role, avatarUrl });

    return {
        status: true,
    };
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
            status: false,
            message: await res.text(),
        };
    }

    return {
        status: true,
        success: await res.json(),
    };
}
