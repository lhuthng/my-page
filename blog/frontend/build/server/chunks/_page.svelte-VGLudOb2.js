import "./utils2-B3oc7zZR.js";
import "./clsx-BYt6phfV.js";
import "./escaping-DoGxUxJF.js";
import "./context-BKhPkoFN.js";
import { ensure_array_like } from "./chunks-CQcQZdVb.js";
import { PostCard } from "./PostCard-EnmsPyMN.js";

//#region .svelte-kit/adapter-bun/entries/pages/posts/_page.svelte.js
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		let { data } = $$props;
		let posts = data.status === "success" ? [...data.featured_posts] : [];
		data.firstOffset;
		$$renderer2.push(`<div class="bg-white/90 space-y-4 rounded-xl p-4 mb-2 lg:mb-4"><ul class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4"><!--[-->`);
		const each_array = ensure_array_like(posts);
		for (let index = 0, $$length = each_array.length; index < $$length; index++) {
			let { title, slug, excerpt, author_name, author_slug, tag_slugs, url } = each_array[index];
			$$renderer2.push(`<li>`);
			PostCard($$renderer2, {
				title,
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
		$$renderer2.push(`<!--]--></ul> <div class="mx-auto w-fit duo-btn duo-blue"><button disabled>No more post</button></div></div>`);
	});
}

//#endregion
export { _page as default };
//# sourceMappingURL=_page.svelte-VGLudOb2.js.map