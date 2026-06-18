import { goto } from '$app/navigation';
import { authState } from '$lib/auth/user.svelte.js';

export function load(event) {
	if (!authState.user) {
	}
}
