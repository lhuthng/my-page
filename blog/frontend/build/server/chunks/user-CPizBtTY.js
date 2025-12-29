import { derived, get, writable } from "./utils-B7r1aIrD.js";

//#region .svelte-kit/adapter-bun/chunks/user.js
let user = writable(void 0);
let isMod = derived(user, ($user) => {
	const role = $user?.role;
	return role === "moderator" || role === "admin";
});
function clearLogin() {
	user.set(void 0);
}
function saveLogin({ username, displayName, token, tokenType, role, avatarUrl }) {
	user.set({
		username,
		displayName,
		role,
		token,
		tokenType,
		avatarUrl
	});
}
function auth() {
	let { token, tokenType } = get(user);
	return `${tokenType} ${token}`;
}

//#endregion
export { auth, clearLogin, isMod, saveLogin, user };
//# sourceMappingURL=user-CPizBtTY.js.map