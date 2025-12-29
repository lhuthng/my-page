import { escape_html } from "./escaping-DoGxUxJF.js";
import { attr, ensure_array_like, store_get, unsubscribe_stores } from "./chunks-CQcQZdVb.js";
import { user } from "./user-CPizBtTY.js";
import { html } from "./html-CocCBXUl.js";

//#region .svelte-kit/adapter-bun/chunks/PostSection.js
function Post($$renderer, $$props) {
	let { content } = $$props;
	$$renderer.push(`<div class="rendered-markdown svelte-s7742d">${html(content)}</div>`);
}
function PostSection($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		var $$store_subs;
		let { id, title, tags, date, content, author } = $$props;
		$$renderer2.push(`<section class="flex not-xl:flex-col max-w-full"><div class="flex grow flex-col bg-white/90 p-4 gap-4 rounded-xl not-xl:rounded-b-none xl:rounded-tr-none"><div class="space-y-2 text-base"><div class="flex gap-4"><h1 class="text-2xl lg:text-4xl">${escape_html(title)}</h1> `);
		if (id && store_get($$store_subs ??= {}, "$user", user)?.username === author.username) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<div class="h-fit duo-btn duo-green"><a${attr("href", `/dashboard/posts/id/${id}`)}>Edit</a></div>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></div> <div class="inline gap-2 text-dark/60">`);
		if (tags?.length > 0) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<ul class="inline text-dark *:inline space-x-1"><!--[-->`);
			const each_array = ensure_array_like(tags);
			for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
				let tag = each_array[$$index];
				$$renderer2.push(`<li class="rounded-full px-1 border-2 border-primary *:no-underline! has-hover:bg-primary duration-100 transition-colors"><a class="text-primary hover:text-white/90 hover:*:text-white/90 duration-100 transition-colors"${attr("href", `/tags/${tag}`)}><span class="text-gray-300">#</span> ${escape_html(tag)}</a></li>`);
			}
			$$renderer2.push(`<!--]--></ul>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></div> <div class="flex items-center gap-2 py-4"><hr class="xl:hidden grow border"/> <span class="text-nowrap">${escape_html(date)}</span> <hr class="grow border"/></div></div> `);
		Post($$renderer2, { content });
		$$renderer2.push(`<!----></div> <div class="min-w-60"><div class="w-full *:full bg-white/90 rounded-xl not-xl:rounded-t-none xl:rounded-l-none"><div class="xl:hidden p-4"><hr class="border"/></div> <div class="pl-4 pt-4">Written by:</div> <div class="flex flex-col gap-2 p-4 pt-2 text-dark"><div class="flex items-center gap-2 bg-secondary/60 p-2 rounded-lg"><div class="w-fit h-fit bg-radial from-white to-secondary rounded-full overflow-hidden"><img class="min-w-16 w-16 h-16 object-contain"${attr("src", author.avatarUrl ?? "/missing.png")} alt="author-avatar"/></div> <div class="flex flex-col"><a class="font-semibold text-dark/80 text-nowrap"${attr("href", `/profiles/${author.username}`)}>${escape_html(author.displayName)}</a> <span>${escape_html(author.username)}</span></div></div></div></div> <svg class="not-xl:hidden w-6 fill-white/90" viewBox="0 0 24 24"><path d="M 0,0 L 12,0 A 12,12 0 0 0 0,12 Z"></path></svg></div></section>`);
		if ($$store_subs) unsubscribe_stores($$store_subs);
	});
}

//#endregion
export { PostSection };
//# sourceMappingURL=PostSection-DxZK4VZX.js.map