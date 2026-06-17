import { goto } from '$app/navigation';
import { user } from '$lib/auth/user';
import { get } from 'svelte/store';

export function load(event) {
	if (!get(user)) {
	}
}
