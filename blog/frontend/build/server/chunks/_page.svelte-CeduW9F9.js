import "./utils2-B3oc7zZR.js";
import "./internal-CyqLiTQC.js";
import "./utils-B7r1aIrD.js";
import "./clsx-BYt6phfV.js";
import { escape_html } from "./escaping-DoGxUxJF.js";
import "./context-BKhPkoFN.js";
import { store_get, unsubscribe_stores } from "./chunks-CQcQZdVb.js";
import "./state.svelte-CeWcm6KF.js";
import { user } from "./user-CPizBtTY.js";

//#region .svelte-kit/adapter-bun/entries/pages/users/_slug_/_page.svelte.js
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		var $$store_subs;
		let { data } = $$props;
		const { username, display_name: displayName, role, bio } = data;
		let isAuthor = ["admin", "moderator"].includes(role) && username === store_get($$store_subs ??= {}, "$user", user)?.username;
		$$renderer2.push(`<div class="w-cap py-20"><div class="w-full space-y-2"><div class="flex"><div class="grow divide-y"><h1 class="text-5xl">${escape_html(displayName)} `);
		if (isAuthor) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<span>(you)</span>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></h1> <p>${escape_html(bio)}</p></div> <div class="w-80 h-80 bg-red-500 rounded-full"></div></div> <div class="w-full h-20 bg-blue-500"><div><div class="flex justify-between"><h2>Posts</h2> `);
		if (isAuthor) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<a href="/dashboard/posts/new">new</a>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></div></div></div></div></div>`);
		if ($$store_subs) unsubscribe_stores($$store_subs);
	});
}

//#endregion
export { _page as default };
//# sourceMappingURL=_page.svelte-CeduW9F9.js.map