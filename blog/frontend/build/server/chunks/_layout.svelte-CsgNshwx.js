import "./utils2-B3oc7zZR.js";
import "./internal-CyqLiTQC.js";
import "./utils-B7r1aIrD.js";
import "./clsx-BYt6phfV.js";
import { escape_html } from "./escaping-DoGxUxJF.js";
import { getContext } from "./context-BKhPkoFN.js";
import { attr, attr_class, attributes, clsx$1 as clsx, element, ensure_array_like, store_get, stringify, unsubscribe_stores } from "./chunks-CQcQZdVb.js";
import "./state.svelte-CeWcm6KF.js";
import { isMod, user } from "./user-CPizBtTY.js";
import { SearchButton } from "./SearchButton-BdwpD_3M.js";

//#region .svelte-kit/adapter-bun/entries/pages/_layout.svelte.js
const getStores = () => {
	const stores$1 = getContext("__svelte__");
	return {
		page: { subscribe: stores$1.page.subscribe },
		navigating: { subscribe: stores$1.navigating.subscribe },
		updated: stores$1.updated
	};
};
const page = { subscribe(fn) {
	const store = getStores().page;
	return store.subscribe(fn);
} };
function Footer($$renderer) {
	$$renderer.push(`<footer class="w-full bg-dark py-2 text-white divide-white"><div class="flex not-sm:flex-col w-cap justify-between"><div class="lg:w-1/2 pt-4 space-y-2 sm:space-y-4 not-sm:text-center"><h2 class="text-2xl"><a href="/"><img class="inline h-10" src="/logo.svg" alt="logo-icon"/></a> Huu Thang's Blog</h2> <p>Exploring the intersection of programming and creativity
                (probably)</p></div> <div class="flex grow justify-evenly gap-12 px-0 lg:px-4 py-2 lg:py-4"><div class="space-y-2 text-left sm:text-right"><h3 class="text-xl">Explore</h3> <ul class="space-y-1 list-inside list-['-'] pt-1 border-t-2 pl-2 sm:pl-0 border-l-2 sm:border-l-0 sm:pr-2 sm:border-r-2"><li><a href="/posts">Posts</a></li> <li><a href="/projects">Projects</a></li> <li><a href="/about">About</a></li></ul></div> <div class="space-y-2 text-right"><h3 class="text-xl">Connect</h3> <ul class="space-y-1 list-inside list-['-'] pt-1 border-t-2 pr-2 border-r-2"><li><a href="https://github.com/lhuthng">Github</a></li> <li><a href="https://www.linkedin.com/in/huuthangle/">Linkedin</a></li> <li><a href="https://www.facebook.com/lhuthng/">Facebook</a></li> <li><a href="https://www.youtube.com/@memofie">Youtube</a></li></ul></div></div></div> <hr class="w-cap border-t-2 my-4"/> <div class="w-cap not-sm:text-center"><span class="mx-auto">© 2025 Huu Thang · Hand-craft with Svelte &amp; Rust <span class="text-nowrap">[Running on a VPS]</span>.</span></div></footer>`);
}
function Header($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		var $$store_subs;
		let displayName = store_get($$store_subs ??= {}, "$user", user)?.displayName;
		let username = store_get($$store_subs ??= {}, "$user", user)?.username;
		store_get($$store_subs ??= {}, "$user", user)?.role;
		let avatarUrl = store_get($$store_subs ??= {}, "$user", user)?.avatarUrl ?? "/missing.png";
		const minLength = 3;
		let searchData = {
			selection: "Post",
			term: "",
			_term: "",
			status: "init",
			result: null
		};
		const search = async (type, term) => {
			if (term.length < minLength) return;
			searchData.status = "fetching";
			searchData.result = void 0;
			{
				const res = await fetch(`/api/posts?term=${encodeURI(term)}&size=5`, { method: "GET" });
				if (res.ok) searchData.result = await res.json();
			}
			searchData.status = "fetched";
		};
		$$renderer2.push(`<header class="fixed w-full bg-white text-dark shadow-lg z-100"><div class="flex not-lg:justify-center w-cap-2 p-2 lg:p-4 gap-4"><div class="flex items-center gap-4"><div class="rounded-full bg-background transition-all duration-200 shadow-lg hover:brightness-102 hover:scale-102"><a href="/"><img class="not-lg:h-10 h-20" src="/logo.svg" alt="logo icon"/></a></div></div> <div class="relative grow not-lg:hidden"><a class="font-semibold text-3xl text-dark transition-all duration-200 hover:scale-102" href="/">Huu Thang's blog</a> <div class="relative z-10 flex max-w-160 h-10 items-center rounded-xl border-2 transition-colors duration-50 bg-dark border-dark overflow-hidden"><div class="relative grow h-full grid px-2 bg-white"><input class="w-full" name="search-input" placeholder="Search" autocomplete="off"${attr("value", searchData._term)}/> <button title="close-btn" class="absolute right-2 top-1/2 -translate-y-1/2 h-full w-fit"><svg class="h-3/5 stroke-dark" viewBox="0 0 24 24"><path id="primary" d="M19,19,5,5M19,5,5,19" style="stroke-linecap: round; stroke-linejoin: round; stroke-width: 2;"></path></svg></button></div> <div class="flex items-center h-full"><div class="h-full px-2 border-l-2 rounded-r-lg bg-white"><select class="full text-dark outline-none" name="search-options">`);
		$$renderer2.option({}, ($$renderer3) => {
			$$renderer3.push(`Post`);
		});
		$$renderer2.option({}, ($$renderer3) => {
			$$renderer3.push(`Tag`);
		});
		$$renderer2.option({}, ($$renderer3) => {
			$$renderer3.push(`User`);
		});
		$$renderer2.push(`</select></div> <div>`);
		SearchButton($$renderer2, {
			class: "p-2 w-10 h-full transition-transform duration-50 active:translate-y-0.5",
			fill: "white",
			onclick: () => search(searchData.selection, searchData._term)
		});
		$$renderer2.push(`<!----></div></div></div> <div class="absolute z-9 left-0 w-full top-full h-fit drop-shadow-sm">`);
		if (searchData.status !== "init") {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<div class="w-full h-fit bg-white rounded-b-xl p-4">`);
			if (searchData.status === "fetching" || searchData.status === "typing") {
				$$renderer2.push("<!--[-->");
				$$renderer2.push(`<span>Loading</span>`);
			} else {
				$$renderer2.push("<!--[!-->");
				if (searchData.status === "fetched") {
					$$renderer2.push("<!--[-->");
					{
						$$renderer2.push("<!--[!-->");
						{
							$$renderer2.push("<!--[!-->");
							{
								$$renderer2.push("<!--[-->");
								if (searchData.result === null || searchData.result.posts.length === 0) {
									$$renderer2.push("<!--[-->");
									$$renderer2.push(`<span>No post matches "${escape_html(searchData.term)}"</span>`);
								} else {
									$$renderer2.push("<!--[!-->");
									$$renderer2.push(`<ul class="space-y-2"><!--[-->`);
									const each_array_1 = ensure_array_like(searchData.result.posts);
									for (let index = 0, $$length = each_array_1.length; index < $$length; index++) {
										let { title, slug, cover_url: coverUrl } = each_array_1[index];
										$$renderer2.push(`<li class="flex gap-4 items-center"><img class="w-10 h-10 object-cover rounded-sm"${attr("src", coverUrl ?? "/missing.png")} alt="mini-avatar"/> <a${attr("href", "posts/" + slug)}>${escape_html(title)}</a></li>`);
									}
									$$renderer2.push(`<!--]--></ul>`);
								}
								$$renderer2.push(`<!--]-->`);
							}
							$$renderer2.push(`<!--]-->`);
						}
						$$renderer2.push(`<!--]-->`);
					}
					$$renderer2.push(`<!--]-->`);
				} else $$renderer2.push("<!--[!-->");
				$$renderer2.push(`<!--]-->`);
			}
			$$renderer2.push(`<!--]--></div>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></div></div> <div class="flex flex-col not-lg:hidden items-end gap-3"><div class="flex h-9 gap-2 items-center text-dark">`);
		if (displayName && username) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<div class="duo-btn duo-primary"><button>Sign Out</button></div>`);
		} else {
			$$renderer2.push("<!--[!-->");
			$$renderer2.push(`<span>wanna</span> <div class="duo-btn duo-primary"><a class="no-underline!" href="/login">Log In</a></div> <span class="text-dark">or</span> <div class="duo-btn duo-primary"><a class="no-underline!" href="/login?register=true">Sign Up?</a></div>`);
		}
		$$renderer2.push(`<!--]--></div> `);
		if (displayName && username) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<div>Welcome back, <a class="font-bold"${attr("href", `/profiles/${stringify(username)}`)}>${escape_html(displayName)} <img class="w-6 h-6 ml-1 rounded-full border-2 inline object-cover"${attr("src", avatarUrl)} alt="small-avatar"/></a></div>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></div></div></header>`);
		if ($$store_subs) unsubscribe_stores($$store_subs);
	});
}
function AboutButton($$renderer, $$props) {
	let { fill, $$slots, $$events,...rest } = $$props;
	$$renderer.push(`<button${attributes({ ...rest }, "svelte-s1dlpv")}><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" x="0px" y="0px"${attr("fill", fill)}><g><path d="M16,16A7,7,0,1,0,9,9,7,7,0,0,0,16,16ZM16,4a5,5,0,1,1-5,5A5,5,0,0,1,16,4Z"></path><path d="M17,18H15A11,11,0,0,0,4,29a1,1,0,0,0,1,1H27a1,1,0,0,0,1-1A11,11,0,0,0,17,18ZM6.06,28A9,9,0,0,1,15,20h2a9,9,0,0,1,8.94,8Z"></path></g></svg></button>`);
}
function BlogButton($$renderer, $$props) {
	let { fill, $$slots, $$events,...rest } = $$props;
	$$renderer.push(`<button${attributes({ ...rest }, "svelte-1f4tz8e")}><svg xmlns="http://www.w3.org/2000/svg" x="0px" y="0px" viewBox="0 0 32 32"${attr("fill", fill)}><path d="M25,27H7c-2.2,0-4-1.8-4-4V9c0-2.2,1.8-4,4-4h18c2.2,0,4,1.8,4,4v14C29,25.2,27.2,27,25,27z" class="svelte-1f4tz8e"></path><rect x="8" y="11" width="6" height="6" class="svelte-1f4tz8e"></rect><line x1="18" y1="17" x2="24" y2="17" class="svelte-1f4tz8e"></line><line x1="8" y1="21" x2="24" y2="21" class="svelte-1f4tz8e"></line></svg></button>`);
}
function DashboardButton($$renderer, $$props) {
	let { fill, $$slots, $$events,...rest } = $$props;
	$$renderer.push(`<button${attributes({ ...rest }, "svelte-67jlc2")}><svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg" x="0px" y="0px"${attr("fill", fill)}><g><path d="M11.5,1.78h-7L1,8l3.5,6.22h7L15,8ZM8,10.7A2.7,2.7,0,1,1,10.7,8,2.7,2.7,0,0,1,8,10.7Z"></path></g></svg></button>`);
}
function FacebookButton($$renderer, $$props) {
	let { as = "button", $$slots, $$events,...rest } = $$props;
	element($$renderer, as, () => {
		$$renderer.push(`${attributes({ ...rest })}`);
	}, () => {
		$$renderer.push(`<svg viewBox="0 0 15.4 15.4" xmlns="http://www.w3.org/2000/svg" x="0px" y="0px"><defs><mask fill="black" id="fb-mask"><path fill="white" d="M0 0h16v16H0z"></path><path d="M8.789 4.723c.387 0 1.041.006 1.041.006V3.415s-.455-.011-1.052 0-2.064.057-2.064 1.786v1.115H5.577v1.365h1.137v4.265H8.34V7.681h1.331l.159-1.365H8.34V5.201s.063-.478.449-.478"></path></mask></defs><path d="M3.209.52h8.941c1.479 0 2.69 1.21 2.69 2.69v8.941c0 1.479-1.21 2.69-2.69 2.69H3.209a2.7 2.7 0 0 1-2.69-2.69V3.21A2.697 2.697 0 0 1 3.209.521Z" mask="url(#fb-mask)"></path></svg>`);
	});
}
function GithubButton($$renderer, $$props) {
	let { as = "button", $$slots, $$events,...rest } = $$props;
	element($$renderer, as, () => {
		$$renderer.push(`${attributes({ ...rest })}`);
	}, () => {
		$$renderer.push(`<svg viewBox="0 0 15.4 15.4" xmlns="http://www.w3.org/2000/svg" x="0px" y="0px"><defs><mask fill="black" id="github-mask"><path fill="white" d="M0 0h16v16H0z"></path><path d="M6.617 12.611c0-.136-.01-.6-.01-1.085-1.615.349-1.951-.697-1.951-.697-.26-.678-.644-.852-.644-.852-.529-.358.039-.358.039-.358.586.039.894.6.894.6.519.891 1.355.639 1.692.484.048-.378.202-.639.365-.784-1.288-.136-2.643-.639-2.643-2.886 0-.639.231-1.162.596-1.569-.058-.145-.26-.746.058-1.549 0 0 .49-.155 1.595.6a5.6 5.6 0 0 1 1.451-.194c.49 0 .99.068 1.451.194 1.105-.755 1.596-.6 1.596-.6.317.804.115 1.404.058 1.549.375.407.596.929.596 1.569 0 2.246-1.355 2.74-2.653 2.886.212.184.394.532.394 1.084 0 .784-.01 1.414-.01 1.607"></path></mask></defs><path mask="url(#github-mask)" d="M3.209.52h8.941c1.479 0 2.69 1.21 2.69 2.69v8.941c0 1.479-1.21 2.69-2.69 2.69H3.209a2.7 2.7 0 0 1-2.69-2.69V3.21A2.697 2.697 0 0 1 3.209.521Z"></path></svg>`);
	});
}
function HomeButton($$renderer, $$props) {
	let { fill, $$slots, $$events,...rest } = $$props;
	$$renderer.push(`<button${attributes({ ...rest }, "svelte-1iu5jtr")}><svg xmlns="http://www.w3.org/2000/svg" x="0px" y="0px" viewBox="0 0 100 100"${attr("fill", fill)}><path d="M83.505,37.85L51.013,12.688c-0.915-0.707-2.197-0.698-3.1,0.025L16.46,37.874c-0.592,0.474-0.939,1.195-0.939,1.956v45.5 c0,1.385,1.121,2.505,2.505,2.505h18.697c1.382,0,2.505-1.121,2.505-2.505V57.471h21.54V85.33c0,1.385,1.123,2.505,2.505,2.505h18.7 c1.382,0,2.505-1.121,2.505-2.505v-45.5C84.479,39.055,84.119,38.324,83.505,37.85z"></path></svg></button>`);
}
function LinkedinButton($$renderer, $$props) {
	let { as = "button", $$slots, $$events,...rest } = $$props;
	element($$renderer, as, () => {
		$$renderer.push(`${attributes({ ...rest })}`);
	}, () => {
		$$renderer.push(`<svg viewBox="0 0 15.4 15.4" xmlns="http://www.w3.org/2000/svg" x="0px" y="0px"><defs><mask fill="black" id="linkedin-mask"><path fill="white" d="M0 0h16v16H0z"></path><path d="M5.92 11.679a.28.28 0 0 1-.281.281H4.443a.28.28 0 0 1-.281-.281V6.666c0-.155.126-.281.281-.281h1.196c.155 0 .281.126.281.281zm-.879-5.766a1.137 1.137 0 1 1 .001-2.273 1.137 1.137 0 0 1-.001 2.273m7.128 5.789a.26.26 0 0 1-.258.258h-1.283a.26.26 0 0 1-.258-.258V9.351c0-.351.103-1.537-.917-1.537-.791 0-.951.812-.983 1.176v2.712a.26.26 0 0 1-.258.258H6.971a.26.26 0 0 1-.258-.258V6.644c0-.143.116-.258.258-.258h1.241c.143 0 .258.116.258.258v.437c.293-.44.729-.78 1.657-.78 2.055 0 2.043 1.92 2.043 2.974z"></path></mask></defs><path d="M3.209.52h8.941c1.479 0 2.69 1.21 2.69 2.69v8.941c0 1.479-1.21 2.69-2.69 2.69H3.209a2.7 2.7 0 0 1-2.69-2.69V3.21A2.697 2.697 0 0 1 3.209.521Z" mask="url(#linkedin-mask)"></path></svg>`);
	});
}
function ProjectButton($$renderer, $$props) {
	let { fill, $$slots, $$events,...rest } = $$props;
	$$renderer.push(`<button${attributes({ ...rest }, "svelte-17uhb37")}><svg xmlns="http://www.w3.org/2000/svg" x="0px" y="0px" viewBox="0 0 24 24"${attr("fill", fill)}><g><path d="M6,6H2V2h4V6z M14,2h-4v4h4V2z M22,2h-4v4h4V2z M6,10H2v4h4V10z M14,10h-4v4h4V10z M22,10h-4v4h4V10z M6,18H2v4h4V18z M14,18h-4v4h4V18z M22,18h-4v4h4V18z"></path></g></svg></button>`);
}
function NavigationSideBar($$renderer, $$props) {
	var $$store_subs;
	let { route } = $$props;
	const routes = [
		[
			HomeButton,
			"Home",
			"/",
			""
		],
		[
			BlogButton,
			"Posts",
			"/posts",
			"posts"
		],
		[
			ProjectButton,
			"Projects",
			"/projects",
			"projects"
		],
		[
			AboutButton,
			"About",
			"/about",
			"about"
		],
		[
			DashboardButton,
			"Dashboard",
			"/dashboard",
			"dashboard",
			true
		]
	];
	$$renderer.push(`<div class="relative not-sm:hidden *:w-12 *:lg:w-46"><div></div> <div class="fixed not-lg:top-16 top-32 space-y-4 lg:space-y-8 transition-transform duration-200"><ul class="space-y-2 bg-white/90 p-2 rounded-xl svelte-1c0jnte" id="side-bar"><!--[-->`);
	const each_array = ensure_array_like(routes);
	for (let index = 0, $$length = each_array.length; index < $$length; index++) {
		let [Icon, text, path, routeName, secret] = each_array[index];
		if (!secret || store_get($$store_subs ??= {}, "$isMod", isMod)) {
			$$renderer.push("<!--[-->");
			$$renderer.push(`<li${attr_class(clsx(routeName === route ? "selected" : void 0), "svelte-1c0jnte")}><a${attr("href", path)} class="svelte-1c0jnte"><!---->`);
			Icon($$renderer, { class: "w-8 transition-all duration-100" });
			$$renderer.push(`<!----> <span class="not-lg:hidden">${escape_html(text)}</span></a></li>`);
		} else $$renderer.push("<!--[!-->");
		$$renderer.push(`<!--]-->`);
	}
	$$renderer.push(`<!--]--></ul> <div class="flex flex-col gap-4 bg-white/90 p-1 lg:p-2 rounded-xl"><div class="not-lg:w-10 flex flex-col"><span class="block not-lg:hidden text-center">Connect with me on:</span> <div class="flex w-full not-lg:flex-col lg:justify-evenly items-center [&amp;>a]:hover:fill-black/90">`);
	FacebookButton($$renderer, {
		as: "a",
		class: "w-10 fill-dark",
		href: "https://www.facebook.com/lhuthng/"
	});
	$$renderer.push(`<!----> `);
	GithubButton($$renderer, {
		as: "a",
		class: "w-10 fill-dark",
		href: "https://github.com/lhuthng"
	});
	$$renderer.push(`<!----> `);
	LinkedinButton($$renderer, {
		as: "a",
		class: "w-10 fill-dark",
		href: "https://www.linkedin.com/in/huuthangle/"
	});
	$$renderer.push(`<!----></div></div> <div class="not-lg:hidden px-2 flex flex-col font-medium"><span class="font-normal">more:</span> <ul class="list-disc list-inside"><li><a href="https://portfolio.huuthang.site" class="svelte-1c0jnte">Portfolio</a></li> <li><a href="/" class="svelte-1c0jnte">About</a></li></ul></div></div></div></div>`);
	if ($$store_subs) unsubscribe_stores($$store_subs);
}
function _layout($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		var $$store_subs;
		let { children } = $$props;
		let route = store_get($$store_subs ??= {}, "$page", page).url.pathname.split("/")[1];
		$$renderer2.push(`<div class="relative flex flex-col min-h-screen"><div class="absolute pointer-events-none inset-0 z-50"></div> `);
		Header($$renderer2);
		$$renderer2.push(`<!----> <main class="grow svelte-12qhfyh"><div class="relative flex gap-2 lg:gap-4 w-cap">`);
		if (route !== "login") {
			$$renderer2.push("<!--[-->");
			NavigationSideBar($$renderer2, { route });
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--> <div class="w-full">`);
		children?.($$renderer2);
		$$renderer2.push(`<!----></div></div></main> `);
		Footer($$renderer2);
		$$renderer2.push(`<!----></div>`);
		if ($$store_subs) unsubscribe_stores($$store_subs);
	});
}

//#endregion
export { _layout as default };
//# sourceMappingURL=_layout.svelte-CsgNshwx.js.map