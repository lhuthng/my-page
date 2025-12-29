import "./utils2-B3oc7zZR.js";
import "./clsx-BYt6phfV.js";
import "./escaping-DoGxUxJF.js";
import "./context-BKhPkoFN.js";
import { attr, attr_class, ensure_array_like, head } from "./chunks-CQcQZdVb.js";
import { PostCard } from "./PostCard-EnmsPyMN.js";

//#region .svelte-kit/adapter-bun/entries/pages/_page.svelte.js
function exploreMore($$renderer, link) {
	$$renderer.push(`<li class="flex justify-center items-center full min-w-22 sm:min-w-26 min-h-22 sm:min-h-26 md:min-w-34 md:min-h-34 rounded-lg border-2 border-dashed"><div class="duo-btn duo-blue"><a class="no-underline!"${attr("href", link)}>explore more</a></div></li>`);
}
function HomeDiscovery($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		const { featuredPosts } = $$props;
		let tab = { index: 1 };
		$$renderer2.push(`<div class="space-y-4 pt-4"><div class="flex justify-between"><ul id="home-tab" class="text-xl font-medium h-8 svelte-183yxu1"><li${attr_class("svelte-183yxu1", void 0, {
			"left": true,
			"selected": tab.index === 1
		})}><button class="svelte-183yxu1">Discover</button></li> <li${attr_class("svelte-183yxu1", void 0, {
			"right": true,
			"selected": tab.index === 2
		})}><button class="svelte-183yxu1">Fresh</button></li></ul></div> <div class="pb-2"><ul class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4"><!--[-->`);
		const each_array = ensure_array_like(featuredPosts);
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
		$$renderer2.push(`<!--]--> `);
		exploreMore($$renderer2, "/posts");
		$$renderer2.push(`<!----></ul> <ul class="hidden grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4">`);
		{
			$$renderer2.push("<!--[!-->");
			$$renderer2.push(`<div class="w-full col-span-full py-10 text-center">Loading</div>`);
		}
		$$renderer2.push(`<!--]--></ul></div></div>`);
	});
}
function Introduction($$renderer) {
	$$renderer.push(`<div class="w-full bg-black text-background text-lg space-y-4 rounded-xl p-2"><div class="relative flex not-sm:flex-col"><div class="not-sm:absolute z-9 min-w-full sm:min-w-[max(16rem,20%)] not-sm:h-full rounded-lg overflow-hidden"><div class="absolute w-full not-sm:-top-20 not-sm:opacity-80 sm:static sm:flex mask-b-from-10% sm:mask-r-from-50% sm:mask-r-to-80%"><img class="w-full h-80 object-cover sm:object-contain object-top-left brightness-110 my-auto" src="/me-1.jpg" alt="me"/></div></div> <div class="relative z-10 not-md:px-2 [&amp;>p]:text-base [&amp;>p]:lg:text-lg space-y-2"><h2 class="text-xl md:text-2xl pt-4 pb-2">Welcome to the <strong>Field</strong>!</h2> <p>I’m <a href="/profiles/thnglhu">Thắng</a>, and this is my
                digital garden. This site serves as a personal archive where I
                document my work experiences, my evolving hobbies, and the
                various experiments I run in my spare time.</p> <p>Everything you see here is a reflection of my curiosity. Whether
                it's a deep dive into a technical challenge or a small creative
                spark, I wanted a central place to capture the process.</p></div></div> <div class="flex flex-col w-full max-w-200 mx-auto px-2 gap-10 text-white text-base lg:text-lg"><p class="md:w-3/5">To navigate the diversity of these projects and keep the content
            organized for different interests, I’ve categorized my contributions
            under <span class="text-red-400">three distinct personas</span>:</p> <div class="flex flex-col gap-4 relative"><img class="mx-auto w-100 py-2" src="/thinkcats.jpg" alt="roles"/> <ul class="full md:left-1/2 md:-translate-x-1/2 block md:absolute md:top-0 *:block md:*:absolute not-md:*:w-full not-md:space-y-2 text-shadow-lg"><li class="left-3/5 w-2/5"><span class="inline-block md:text-right"><a class="bg-red-500 text-white rounded-full py-1 px-2" href="/profiles/admin">The Architect</a>: Focused on the system's structure, the "how-to," and
                        overall management of the platform.</span></li> <li class="left-0 bottom-20 w-2/7"><span><a class="bg-red-500 text-white rounded-full py-1 px-2" href="/profiles/thnglhu">Thắng</a>: My personal account for sharing direct experiences,
                        unfiltered thoughts, and hands-on experiments.</span></li> <li class="right-0 bottom-20 w-2/7"><span class="inline-block md:text-right"><a class="bg-red-500 text-white rounded-full py-1 px-2" href="/profiles/lhuthng">Memory Field</a>: A dedicated space for my creative works, artistic
                        side projects, and the "juice" that makes the work feel
                        alive.</span></li></ul></div> <span class="inline-block ml-auto">by <a class="text-background" href="/profiles/lhuthng">@lhuthng</a></span></div></div>`);
}
function Suggestion($$renderer) {
	$$renderer.push(`<div class="w-60 px-4 py-2"><h3 class="font-semibold">To do list (for this blog):</h3> <ul class="list-disc list-inside"><li>Fill post content</li> <li>Add Series</li> <li>Post filter</li> <li>Fix the mail service for registering</li> <li>Change the ugly footer</li> <li>Redraw the logo</li> <li>Add the Projects tab</li> <li>Add the About tab</li></ul></div>`);
}
function _page($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		const { data } = $$props;
		const featuredPosts = data.featured_posts || [];
		head("1uha8ag", $$renderer2, ($$renderer3) => {
			$$renderer3.title(($$renderer4) => {
				$$renderer4.push(`<title>Home</title>`);
			});
		});
		$$renderer2.push(`<div class="relative z-5 flex gap-4 *:h-fit pb-2 lg:pb-4"><div class="grow space-y-2 lg:space-y-4"><div class="bg-white/90 rounded-xl px-4 overflow-hidden">`);
		HomeDiscovery($$renderer2, { featuredPosts });
		$$renderer2.push(`<!----></div></div> <div class="not-lg:hidden w-60 bg-white/90 rounded-xl">`);
		Suggestion($$renderer2);
		$$renderer2.push(`<!----></div></div> <div class="pb-4">`);
		Introduction($$renderer2);
		$$renderer2.push(`<!----></div>`);
	});
}

//#endregion
export { _page as default };
//# sourceMappingURL=_page.svelte-DE5aJopw.js.map