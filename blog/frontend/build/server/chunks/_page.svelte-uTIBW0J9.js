import "./utils2-B3oc7zZR.js";
import "./internal-CyqLiTQC.js";
import "./utils-B7r1aIrD.js";
import "./clsx-BYt6phfV.js";
import { escape_html } from "./escaping-DoGxUxJF.js";
import "./context-BKhPkoFN.js";
import { attr, ensure_array_like, store_get, unsubscribe_stores } from "./chunks-CQcQZdVb.js";
import "./state.svelte-CeWcm6KF.js";
import { user } from "./user-CPizBtTY.js";
import { PostCard } from "./PostCard-EnmsPyMN.js";

//#region .svelte-kit/adapter-bun/entries/pages/dashboard/posts/_page.svelte.js
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		var $$store_subs;
		store_get($$store_subs ??= {}, "$user", user).username;
		let posts = {
			fetchedAll: false,
			data: []
		};
		$$renderer2.push(`<section class="flex flex-col gap-4 *:bg-white/90 *:rounded-xl *:p-4 pb-8"><div class="flex flex-col gap-4"><ul class="grid grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4"><!--[-->`);
		const each_array = ensure_array_like(posts.data);
		for (let index = 0, $$length = each_array.length; index < $$length; index++) {
			let { title, slug, id, excerpt, author_name, author_slug, tag_slugs, url } = each_array[index];
			$$renderer2.push(`<li>`);
			PostCard($$renderer2, {
				title,
				id,
				slug,
				excerpt,
				author: {
					name: author_name,
					slug: author_slug
				},
				tags: tag_slugs,
				src: url
			});
			$$renderer2.push(`<!----></li>`);
		}
		$$renderer2.push(`<!--]--></ul> <div class="flex col-span-full full items-center justify-center"><div class="duo-btn duo-green"><button${attr("disabled", posts.fetchedAll, true)}>${escape_html("load more")}</button></div></div></div></section>`);
		if ($$store_subs) unsubscribe_stores($$store_subs);
	});
}

//#endregion
export { _page as default };
//# sourceMappingURL=_page.svelte-uTIBW0J9.js.map