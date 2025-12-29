import "./utils2-B3oc7zZR.js";
import "./internal-CyqLiTQC.js";
import "./utils-B7r1aIrD.js";
import "./clsx-BYt6phfV.js";
import { escape_html } from "./escaping-DoGxUxJF.js";
import "./context-BKhPkoFN.js";
import { attr, store_get, unsubscribe_stores } from "./chunks-CQcQZdVb.js";
import "./state.svelte-CeWcm6KF.js";
import { user } from "./user-CPizBtTY.js";

//#region .svelte-kit/adapter-bun/entries/pages/login/_page.svelte.js
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		var $$store_subs;
		let { data } = $$props;
		let isLogged = store_get($$store_subs ??= {}, "$user", user) !== void 0;
		let isLogging = !data.register;
		let username = "";
		let password = "";
		let repassword = "";
		let email = "";
		$$renderer2.push(`<div class="flex w-full items-center min-h-[calc(100dvh-8rem)] py-4"><div class="mx-auto p-8 rounded-3xl bg-white/80"><form class="flex flex-col gap-4 w-80 *:items-center text-xl" novalidate><h3 class="mx-auto py-2 svelte-1x05zx6">`);
		if (isLogging) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`Log In`);
		} else {
			$$renderer2.push("<!--[!-->");
			$$renderer2.push(`Sign Up`);
		}
		$$renderer2.push(`<!--]--></h3> <div class="space-y-2 *:rounded-xl *:border-2 *:border-dark/40 *:has-focus:border-dark *:bg-primary/20 *:has-disabled:opacity-40 text-dark"><div class="px-2"><input class="py-1.5 w-full" placeholder="Username" autocomplete="username"${attr("value", username)}${attr("disabled", isLogged && isLogging, true)}/></div> <div class="flex gap-2 px-2"><input class="grow py-1.5" placeholder="Password" type="password"${attr("value", password)}${attr("disabled", isLogged && isLogging, true)}/> `);
		if (isLogging) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<button type="button" class="text-primary/80 hover:text-dark cursor-pointer"${attr("disabled", isLogged, true)}>forgot?</button>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></div> `);
		if (!isLogging) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<div class="flex px-2"><input class="grow py-1.5" placeholder="Re-password" type="password"${attr("value", repassword)}/></div> <div class="flex px-2"><input class="grow py-1.5" placeholder="Email" type="email"${attr("value", email)}/></div>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></div> `);
		$$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--> <div class="w-full duo-btn duo-primary"><button class="w-full" type="submit"${attr("disabled", isLogging && isLogged, true)}>`);
		if (isLogging) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`Log In`);
		} else {
			$$renderer2.push("<!--[!-->");
			$$renderer2.push(`Sign Up`);
		}
		$$renderer2.push(`<!--]--></button></div> `);
		if (isLogging && isLogged) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<div class="flex w-full"><span class="mx-auto text-center text-accent-green">You're are logged as ${escape_html(store_get($$store_subs ??= {}, "$user", user).username)}</span></div>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--> <div class="separator svelte-1x05zx6"><span>or</span></div> <div class="w-full duo-btn duo-primary"><button type="button" class="w-full">`);
		if (isLogging) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`Sign Up`);
		} else {
			$$renderer2.push("<!--[!-->");
			$$renderer2.push(`Log In`);
		}
		$$renderer2.push(`<!--]--></button></div> <p class="text-lg text-dark/80 text-justify">You don't need to log in to read posts! Create a profile for a
                cool avatar when you comment. <span class="text-nowrap">𐔌՞ ܸ.ˬ.ܸ՞𐦯</span></p> `);
		if (!isLogging) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<p class="text-lg text-dark/80 text-justify">Passwords are hashed using a one-way function and never
                    stored in plain text. When you log in, the password you
                    enter is hashed and compared to the stored hash - the
                    original password is never stored or recoverable <span class="text-nowrap">ヾ(•̀ ヮ &lt;)و</span>.</p>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></form></div></div>`);
		if ($$store_subs) unsubscribe_stores($$store_subs);
	});
}

//#endregion
export { _page as default };
//# sourceMappingURL=_page.svelte-DMM82yhY.js.map