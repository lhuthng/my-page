import "./utils2-B3oc7zZR.js";
import "./internal-CyqLiTQC.js";
import "./utils-B7r1aIrD.js";
import "./clsx-BYt6phfV.js";
import { escape_html } from "./escaping-DoGxUxJF.js";
import "./context-BKhPkoFN.js";
import { attr, ensure_array_like, store_get, unsubscribe_stores } from "./chunks-CQcQZdVb.js";
import "./state.svelte-CeWcm6KF.js";
import { user } from "./user-CPizBtTY.js";
import { textToDate } from "./common-BLrvVRDf.js";
import "./html-CocCBXUl.js";
import { PostSection } from "./PostSection-DxZK4VZX.js";
import { Club, Diamond, Heart, Spade } from "./Spade-BGQvvZzk.js";

//#region .svelte-kit/adapter-bun/entries/pages/posts/_slug_/_page.svelte.js
function comment_snippet($$renderer, content, username, displayName, date, avatarUrl, userRole) {
	const someone = username !== void 0 && displayName;
	$$renderer.push(`<li class="flex py-4"><div class="ml-2 min-w-10 lg:min-w-12 w-10 lg:w-12 h-10 lg:h-12 rounded-full shadow-md overflow-hidden">`);
	if (someone) {
		$$renderer.push("<!--[-->");
		$$renderer.push(`<a class="full"${attr("href", "/profiles/" + username)}><img class="full object-cover"${attr("src", avatarUrl ?? "/missing.png")} alt="comment-posting-avatar"/></a>`);
	} else {
		$$renderer.push("<!--[!-->");
		$$renderer.push(`<img class="full object-cover"${attr("src", avatarUrl ?? "/missing.png")} alt="comment-posting-avatar"/>`);
	}
	$$renderer.push(`<!--]--></div> <div class="relative flex flex-col grow"><div class="pl-2 -translate-y-2"><div class="relative w-fit"><div class="w-fit p-2 bg-primary/20 rounded-2xl rounded-tl-md"><div class="flex items-center lg:text-base">`);
	if (someone) {
		$$renderer.push("<!--[-->");
		$$renderer.push(`<a class="font-semibold"${attr("href", "/profiles/" + username)}>${escape_html(displayName)}</a>`);
	} else {
		$$renderer.push("<!--[!-->");
		$$renderer.push(`<span class="font-normal select-none italic">Anonymous</span>`);
	}
	$$renderer.push(`<!--]--> <span class="*:w-8 hover:*:translate-x-1 *:transition-all *:duration-200 tooltip-container"${attr("data-tooltip", userRole === "admin" ? "he's THE admin!" : userRole === "moderator" ? "a mod!" : userRole === "user" ? "user" : "?")}>`);
	if (userRole === "admin") {
		$$renderer.push("<!--[-->");
		Heart($$renderer, { class: "fill-accent-red" });
	} else {
		$$renderer.push("<!--[!-->");
		if (userRole === "moderator") {
			$$renderer.push("<!--[-->");
			Diamond($$renderer, { class: "fill-accent-red" });
		} else {
			$$renderer.push("<!--[!-->");
			if (userRole === "user") {
				$$renderer.push("<!--[-->");
				Club($$renderer, { class: "fill-dark" });
			} else {
				$$renderer.push("<!--[!-->");
				Spade($$renderer, { class: "fill-dark" });
			}
			$$renderer.push(`<!--]-->`);
		}
		$$renderer.push(`<!--]-->`);
	}
	$$renderer.push(`<!--]--></span></div> <p class="w-fit min-h-6 whitespace-pre">${escape_html(content)}</p></div> <div class="absolute flex min-w-20 w-full justify-between gap-2 left-0 top-full text-sm"><span class="pl-2">${escape_html(date ?? "new")}</span> <div class="text-nowrap *:cursor-pointer"><span>reply</span> <span>report</span></div></div></div></div></div></li>`);
}
function CommentSection($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		var $$store_subs;
		let userAvatarUrl = store_get($$store_subs ??= {}, "$user", user)?.avatarUrl ?? "/missing.png";
		let comments = {
			current: "",
			fetching: false,
			data: []
		};
		$$renderer2.push(`<section class="w-full xl:w-[calc(100%-15rem)] bg-white/90 p-4 rounded-xl"><h4 class="text-lg lg:text-2xl">Join the discussion!</h4> <div class="flex flex-col gap-4"><hr class="border-t-3 border-dark mb-6"/> <div class="flex gap-8"><div class="w-12 lg:w-20 h-12 lg:h-20 border-dark border-2 rounded-full overflow-hidden"><img class="full object-cover"${attr("src", userAvatarUrl)} alt="comment-posting-avatar"/></div> <div class="grow flex flex-col gap-4 relative"><svg class="absolute fill-primary/60 top-6 lg:top-10 -left-4 -translate-y-1/2 w-4 h-4" viewBox="0 0 24 24"><polygon points="0,12 24,0 24,24"></polygon></svg> <textarea name="comment-input" class="text-base w-full min-h-10 lg:min-h-20 overflow-hidden resize-none outline-dark bg-primary/20 outline-2 p-2 rounded-xl">`);
		const $$body = escape_html(comments.current);
		if ($$body) $$renderer2.push(`${$$body}`);
		$$renderer2.push(`</textarea> <div class="ml-auto w-fit duo-btn duo-blue"><button class="fill-white" type="button"${attr("disabled", comments.current.length < 1, true)}><svg class="w-6 -scale-x-100 -translate-y-0.5 inline-block" viewbox="0 0 24 24"><g><path fill-rule="evenodd" clip-rule="evenodd" d="M3.3572 3.23397C3.66645 2.97447 4.1014 2.92638 4.45988 3.11204L20.7851 11.567C21.1426 11.7522 21.3542 12.1337 21.322 12.5351C21.2898 12.9364 21.02 13.2793 20.6375 13.405L13.7827 15.6586L10.373 22.0179C10.1828 22.3728 9.79826 22.5789 9.39743 22.541C8.9966 22.503 8.65762 22.2284 8.53735 21.8441L3.04564 4.29872C2.92505 3.91345 3.04794 3.49346 3.3572 3.23397ZM5.67123 5.99173L9.73507 18.9752L12.2091 14.361C12.3304 14.1347 12.5341 13.9637 12.7781 13.8835L17.7518 12.2484L5.67123 5.99173Z"></path></g></svg> Send</button></div></div></div></div> <ul><!--[-->`);
		const each_array = ensure_array_like(comments.data);
		for (let index = 0, $$length = each_array.length; index < $$length; index++) {
			let { id, avatar_url, content, created_at, display_name, username, user_role } = each_array[index];
			comment_snippet($$renderer2, content, username, display_name, created_at, avatar_url, user_role);
		}
		$$renderer2.push(`<!--]--></ul> <div class="mt-8 mx-auto w-fit duo-btn duo-blue"><button${attr("disabled", comments.fetching, true)}>${escape_html("Read more")}</button></div></section>`);
		if ($$store_subs) unsubscribe_stores($$store_subs);
	});
}
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		let { data, slug } = $$props;
		let { id, author_slug, author_name, author_avatar_url, title, content, published_at, tags } = data;
		let date = textToDate(published_at);
		$$renderer2.push(`<article class="flex flex-col gap-4 pb-4 *:drop-shadow-xl">`);
		PostSection($$renderer2, {
			id,
			title,
			tags,
			date,
			content,
			author: {
				username: author_slug,
				displayName: author_name,
				avatarUrl: author_avatar_url
			}
		});
		$$renderer2.push(`<!----> `);
		CommentSection($$renderer2);
		$$renderer2.push(`<!----></article>`);
	});
}

//#endregion
export { _page as default };
//# sourceMappingURL=_page.svelte-Du_4sJSS.js.map