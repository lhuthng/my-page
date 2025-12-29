import "./utils2-B3oc7zZR.js";
import "./internal-CyqLiTQC.js";
import "./utils-B7r1aIrD.js";
import "./clsx-BYt6phfV.js";
import { escape_html } from "./escaping-DoGxUxJF.js";
import "./context-BKhPkoFN.js";
import { attr, attr_class, store_get, unsubscribe_stores } from "./chunks-CQcQZdVb.js";
import "./state.svelte-CeWcm6KF.js";
import { user } from "./user-CPizBtTY.js";
import { Club, Diamond, Heart, Spade } from "./Spade-BGQvvZzk.js";

//#region .svelte-kit/adapter-bun/entries/pages/profiles/_slug_/_page.svelte.js
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		var $$store_subs;
		const { data } = $$props;
		let response = data.response;
		let username = response.username;
		let role = response.role;
		let displayName = "";
		let bio = "";
		let avatarUrl = "/missing.png";
		let editor = {
			isEditing: false,
			isFetching: false
		};
		const hasPosts = role === "moderator" || role === "admin";
		$$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--> <section class="flex flex-col gap-4 *:bg-white/90 *:rounded-xl *:p-4 pb-8"${attr("key", username)}><div class="flex gap-4"><div class="space-y-4 min-w-60"><div class="relative rounded-bl-xl rounded-tr-xl overflow-hidden">`);
		if (store_get($$store_subs ??= {}, "$user", user)?.username === username) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<div class="absolute left-0 bottom-0 w-full h-full opacity-0 hover:opacity-100 bg-background/30 transition-opacity duration-200"><div class="absolute bottom-0 left-0 flex items-center justify-center w-full py-2 bg-dark/30"><div class="duo-btn duo-green"><button>Change Avatar</button></div></div></div>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--> <img class="w-60 h-60 object-cover"${attr("src", avatarUrl)} alt="avatar"/></div> <div class="grid grid-cols-2 gap-2"><div class="duo-btn duo-blue"><button disabled>Message</button></div> <div class="duo-btn duo-green"><button disabled>Follow</button></div></div></div> <div class="w-full"><div class="flex items-end">`);
		{
			$$renderer2.push("<!--[!-->");
			$$renderer2.push(`<h2 class="inline font-bold text-3xl">${escape_html(displayName)}</h2>`);
		}
		$$renderer2.push(`<!--]--> <span class="*:w-10 hover:*:translate-x-1 *:transition-all *:duration-200 tooltip-container"${attr("data-tooltip", role)}>`);
		if (role === "admin") {
			$$renderer2.push("<!--[-->");
			Heart($$renderer2, { class: "fill-accent-red" });
		} else {
			$$renderer2.push("<!--[!-->");
			if (role === "moderator") {
				$$renderer2.push("<!--[-->");
				Diamond($$renderer2, { class: "fill-accent-red" });
			} else {
				$$renderer2.push("<!--[!-->");
				if (role === "user") {
					$$renderer2.push("<!--[-->");
					Club($$renderer2, { class: "fill-dark" });
				} else {
					$$renderer2.push("<!--[!-->");
					Spade($$renderer2, { class: "fill-dark" });
				}
				$$renderer2.push(`<!--]-->`);
			}
			$$renderer2.push(`<!--]-->`);
		}
		$$renderer2.push(`<!--]--></span> `);
		if (store_get($$store_subs ??= {}, "$user", user)?.username === username) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<span class="grow"></span> <div${attr_class("duo-btn col-span-full", void 0, {
				"duo-green": true,
				"duo-red": editor.isEditing
			})}><button${attr("disabled", editor.isFetching, true)}>${escape_html("Edit")}</button></div>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></div> <span class="italic text-primary/80 *:select-none"><span>./</span>${escape_html(username)}<span>/</span></span> `);
		{
			$$renderer2.push("<!--[!-->");
			$$renderer2.push(`<p class="py-4 text-lg whitespace-pre-wrap">${escape_html(bio)}</p>`);
		}
		$$renderer2.push(`<!--]--></div></div> `);
		if (hasPosts) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<div class="flex flex-col gap-4"><h2 class="inline font-bold text-2xl">Posts</h2> `);
			{
				$$renderer2.push("<!--[!-->");
				{
					$$renderer2.push("<!--[!-->");
					$$renderer2.push("<!--[!-->");
					$$renderer2.push(`<!--]-->`);
				}
				$$renderer2.push(`<!--]-->`);
			}
			$$renderer2.push(`<!--]--></div>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></section>`);
		if ($$store_subs) unsubscribe_stores($$store_subs);
	});
}

//#endregion
export { _page as default };
//# sourceMappingURL=_page.svelte-DcYeotim.js.map