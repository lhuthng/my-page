import { goto } from "$app/navigation";
import { user } from "$lib/client/user.js";
import { get } from "svelte/store";

export function load(event) {
	if (!get(user)) {
	}
}
