import { writable } from "./utils-B7r1aIrD.js";

//#region .svelte-kit/adapter-bun/chunks/client.js
function create_updated_store() {
	const { set, subscribe } = writable(false);
	return {
		subscribe,
		check: async () => false
	};
}
const stores = { updated: /* @__PURE__ */ create_updated_store() };
stores.updated.check;

//#endregion
//# sourceMappingURL=client-C-eUpif6.js.map