import { escape_html } from "./escaping-DoGxUxJF.js";
import { attr, attr_class, ensure_array_like } from "./chunks-CQcQZdVb.js";

//#region .svelte-kit/adapter-bun/chunks/PostCard.js
function PostCard($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		let { src, id, title, slug, series, excerpt, author, tags, previewMode, children } = $$props;
		let toggled = false;
		let dashboardMode = id !== void 0;
		let link = dashboardMode ? `/dashboard/posts/${id}` : `/posts/${slug}`;
		let postLink = `/posts/${slug}`;
		$$renderer2.push(`<div class="relative flex gap-4 bg-background/40 hover:bg-background/60 transition-colors duration-50 rounded-lg"><a class="relative block z-10 min-w-22 sm:min-w-26 min-h-22 sm:min-h-26 md:min-w-34 md:min-h-34"${attr("href", link)}><img class="absolute z-10 left-0 top-0 w-22 sm:w-26 h-22 sm:h-26 md:w-34 md:h-34 object-cover rounded-lg origin-center transition-transform duration-100 cursor-pointer hover:scale-105"${attr("src", src ?? "/missing.png")} alt="thumbnail"/> `);
		if (children !== void 0) {
			$$renderer2.push("<!--[-->");
			children($$renderer2);
			$$renderer2.push(`<!---->`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></a> <div class="relative z-10 w-full pointer-events-none overflow-hidden"><div${attr_class("card absolute w-[200%] h-full flex transition-transform duration-200 left-0 svelte-pzce7s", void 0, { "toggled": toggled })}><div class="h-full w-1/2 min-w-0"><div class="flex flex-col full py-2 min-w-0"><a class="w-fit"${attr("href", link)}><h1 class="text-sm sm:text-md md:text-lg line-clamp-2">${escape_html(title)} `);
		if (dashboardMode) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`(dashboard)`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></h1></a> <div class="flex text-xs sm:text-sm pr-4"><span class="select-none pointer-events-auto svelte-pzce7s">by <a class="select-text text-dark!"${attr("href", `/profiles/${author.slug}`)}>${escape_html(author.name)}</a></span> `);
		if (series !== void 0) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<div class="flex grow shrink gap-2"><span class="svelte-pzce7s">;</span> <span class="pointer-events-auto svelte-pzce7s"><span class="svelte-pzce7s">::${escape_html(series.order)}</span> from <a class="text-dark!"${attr("href", `/series/${series.slug}`)}>${escape_html(series.name)}</a></span></div>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></div> <div class="flex gap-1 grow shrink text-xs sm:text-sm line-clamp-2">`);
		if (tags?.length > 0) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<span class="svelte-pzce7s">tags:</span>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--> <ul class="tag-container flex flex-wrap h-fit gap-y-1 gap-x-1 pr-2 svelte-pzce7s"><!--[-->`);
		const each_array = ensure_array_like(tags);
		for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
			let tag = each_array[$$index];
			$$renderer2.push(`<li class="svelte-pzce7s"><a${attr("href", `/tags/${tag.replace(" ", "-")}`)}>#${escape_html(tag)}</a></li>`);
		}
		$$renderer2.push(`<!--]--></ul></div> <div class="flex justify-between items-center text-base pr-4"><div class="flex gap-1 items-center text-sm"></div></div></div></div> <div${attr_class("absolute left-1/2 h-full w-1/2", void 0, { "toggled": toggled })}><div class="full p-2 pointer-events-auto text-xs sm:text-base overscroll-contain custom-scrollbar overflow-y-scroll"><p>${escape_html(excerpt)}</p> <a class="block text-right"${attr("href", postLink)}>> to post</a></div></div> <svg${attr_class("card-btn absolute top-0 left-1/2 h-full -translate-x-2/5 has-hover:-translate-x-1/2 has-hover:fill-primary/60 fill-primary/20 transition-all duration-200 z-9 svelte-pzce7s", void 0, { "toggled": toggled })} xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><polygon class="left pointer-events-auto cursor-pointer focus:outline-none svelte-pzce7s" points="16,0 16,32 0,16" role="button" tabindex="0"></polygon><polygon class="right pointer-events-auto cursor-pointer focus:outline-none svelte-pzce7s" points="16,0 16,32 32,16" role="button" tabindex="0"></polygon></svg></div></div></div>`);
	});
}

//#endregion
export { PostCard };
//# sourceMappingURL=PostCard-EnmsPyMN.js.map