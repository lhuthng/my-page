import { escape_html } from "./escaping-DoGxUxJF.js";
import { attr, attr_class, attributes, ensure_array_like, store_get, unsubscribe_stores } from "./chunks-CQcQZdVb.js";
import { user } from "./user-CPizBtTY.js";
import { SearchButton } from "./SearchButton-BdwpD_3M.js";
import { PostCard } from "./PostCard-EnmsPyMN.js";
import { nowToDate } from "./common-BLrvVRDf.js";
import { mediaWithShortcutPlugin, youtubeBlockPlugin } from "./index2-DUFc1Uun.js";
import { PostSection } from "./PostSection-DxZK4VZX.js";
import MarkdownIt from "markdown-it";

//#region .svelte-kit/adapter-bun/chunks/PostEditor.js
function ContentDebounceEdtior($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		let { delay, onUpdateRendered, onUpdateDraft, mediaDictionary, searchMedia, mediaSyntax, registerForceContent, forDraft, disabled, $$slots, $$events,...rest } = $$props;
		let _content = "";
		registerForceContent((content) => {
			_content = content;
		});
		new MarkdownIt().use(mediaWithShortcutPlugin, { mediaDictionary }).use(youtubeBlockPlugin);
		$$renderer2.push(`<div${attributes({ ...rest })}><textarea id="content-editor"${attr_class("w-full h-full p-2 rounded-sm bg-white/60 focus:outline-0 resize-none custom-scrollbar", void 0, { "bg-transparent!": disabled })}${attr("placeholder", disabled ? "" : "Type here...")} autocorrect="off"${attr("disabled", disabled, true)}>`);
		const $$body = escape_html(_content);
		if ($$body) $$renderer2.push(`${$$body}`);
		$$renderer2.push(`</textarea></div>`);
	});
}
function MediumEntity($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		let { shortName, url, warning, changeName } = $$props;
		let draft = shortName;
		$$renderer2.push(`<li${attr_class("flex items-center gap-2 p-2 rounded-lg bg-primary/20 hover:brightness-105", void 0, { "bg-yellow-200": warning })}><div class="flex w-8 lg:w-10 h-8 lg:h-10"><img class="m-auto max-w-8 lg:max-w-10 max-h-8 lg:max-h-10 object-contain rounded-sm overflow-hidden"${attr("src", url)}${attr("alt", shortName)}/></div> `);
		if (changeName) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<input class="focus:outline-none focus:border-b" type="text"${attr("value", draft)}/>`);
		} else {
			$$renderer2.push("<!--[!-->");
			$$renderer2.push(`<span>${escape_html(shortName)}</span>`);
		}
		$$renderer2.push(`<!--]--></li>`);
	});
}
function OfflineMediaCreator($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		let { offlineMedia, onlineMedia, updateNewMedia, changeName, $$slots, $$events,...rest } = $$props;
		$$renderer2.push(`<div${attributes({ ...rest })}><div class="full" role="listitem">`);
		if (Object.keys(offlineMedia).length === 0) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<div class="flex full rounded-lg border-dashed border-2 border-gray-400"><span class="block m-auto">Drop media here</span></div>`);
		} else {
			$$renderer2.push("<!--[!-->");
			$$renderer2.push(`<ul class="full space-y-2"><!--[-->`);
			const each_array = ensure_array_like(Object.keys(offlineMedia).sort().map((key) => ({
				shortName: key,
				url: offlineMedia[key]
			})));
			for (let index = 0, $$length = each_array.length; index < $$length; index++) {
				let { shortName, url } = each_array[index];
				MediumEntity($$renderer2, {
					shortName,
					url,
					changeName,
					warning: shortName in onlineMedia && onlineMedia[shortName]
				});
			}
			$$renderer2.push(`<!--]--></ul>`);
		}
		$$renderer2.push(`<!--]--></div></div>`);
	});
}
function printResults($$renderer, cache, keyword) {
	const results = cache[keyword]?.results;
	$$renderer.push(`<div class="p-2 grow min-h-0 custom-scrollbar">`);
	if (results !== void 0) {
		$$renderer.push("<!--[-->");
		if (results.length === 0) {
			$$renderer.push("<!--[-->");
			$$renderer.push(`<span class="block text-center">There is no medium matching "${escape_html(keyword)}"</span>`);
		} else {
			$$renderer.push("<!--[!-->");
			$$renderer.push(`<ul class="space-y-2"><!--[-->`);
			const each_array = ensure_array_like(results);
			for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
				let { short_name: shortName, url } = each_array[$$index];
				MediumEntity($$renderer, {
					shortName,
					url
				});
			}
			$$renderer.push(`<!--]--></ul>`);
		}
		$$renderer.push(`<!--]-->`);
	} else {
		$$renderer.push("<!--[!-->");
		$$renderer.push(`<span class="block text-center">Enter atleast 2 characters to search media.</span>`);
	}
	$$renderer.push(`<!--]--></div>`);
}
function OnlineMediaSearcher($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		let { $$slots, $$events,...rest } = $$props;
		let cache = {};
		let keyword = "";
		let _keyword = "";
		$$renderer2.push(`<div${attributes({ ...rest })}><div class="mx-auto w-full p-2"><div class="flex items-center rounded-full w-full h-8 px-2 border-2 border-primary"><input id="media-searcher" class="outline-none grow" autocorrect="off"${attr("value", _keyword)}/> `);
		SearchButton($$renderer2, { class: "w-6 h-6 not-disabled:hover:scale-110 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50 fill-primary" });
		$$renderer2.push(`<!----></div></div> `);
		printResults($$renderer2, cache, keyword);
		$$renderer2.push(`<!----></div>`);
	});
}
function MediaDictionaryController($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		let { registerSearch } = $$props;
		let newMedia = {};
		let onlineMedia = {};
		let offlineMedia = Object.fromEntries(Object.entries(newMedia).map(([key, value]) => [key, value.url]));
		const searchOnlineMedia = async (keys) => {
			const dict = { ...onlineMedia };
			const missingKeys = keys.filter((key) => !(key in onlineMedia));
			missingKeys.forEach((key) => dict[key] = null);
			onlineMedia = { ...dict };
			missingKeys.forEach(async (key) => {
				const res = await fetch("/api/media/s/" + key, { method: "GET" });
				if (res.ok) onlineMedia[key] = (await res.json()).url;
				else {
					onlineMedia[key] = void 0;
					setTimeout(() => delete onlineMedia[key], 5e3);
				}
			});
		};
		registerSearch(searchOnlineMedia);
		const updateNewMedia = (media) => {
			const temp = { ...newMedia };
			const names = [];
			media.forEach((medium) => {
				temp[medium.name] = medium;
				names.push(medium.name);
			});
			searchOnlineMedia(names);
			newMedia = { ...temp };
		};
		const changeName = (oldName, newName) => {
			if (newName in newMedia) return false;
			let temp = { ...newMedia };
			temp[newName] = temp[oldName];
			delete temp[oldName];
			newMedia = { ...temp };
			if (!(newName in onlineMedia)) searchOnlineMedia([newName]);
			console.log({ ...onlineMedia }, { ...newMedia });
			return true;
		};
		OnlineMediaSearcher($$renderer2, { class: "flex flex-col w-1/2 lg:w-60 h-full text-dark bg-primary/40 rounded-xl" });
		$$renderer2.push(`<!----> `);
		OfflineMediaCreator($$renderer2, {
			onlineMedia,
			offlineMedia,
			updateNewMedia,
			changeName,
			class: "flex flex-col w-1/2 lg:w-60 h-full text-dark bg-primary/40 rounded-xl"
		});
		$$renderer2.push(`<!---->`);
	});
}
function PostEditor($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		var $$store_subs;
		const mediaSyntax = /\@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]/g;
		let { mode = "create", data } = $$props;
		let mediaDictionary = {};
		let searchMedia = async (keyword) => {};
		let editingData = {
			id: "",
			title: "",
			_slugStatus: {},
			slug: "",
			tags: "",
			excerpt: "",
			date: nowToDate(),
			content: "",
			draft: "",
			author: {
				username: store_get($$store_subs ??= {}, "$user", user)?.username,
				displayName: store_get($$store_subs ??= {}, "$user", user)?.displayName,
				avatarUrl: store_get($$store_subs ??= {}, "$user", user)?.avatarUrl
			}
		};
		let editor = {
			toggled: false,
			view: "private"
		};
		let renderedText = "";
		let forDraft = mode === "create" || mode === "edit" && editor.view === "private";
		if (mode === "edit" && data !== void 0) {
			let { id, title, slug, content, draft, excerpt, tags, mediumShortNames, mediumUrls } = data;
			editingData.id = id;
			editingData.title = title;
			editingData.slug = slug;
			editingData.excerpt = excerpt;
			editingData.tags = tags.join(" ");
			editingData.content = content;
			editingData.draft = draft;
			editor.view = "public";
		}
		$$renderer2.push(`<article class="relative flex flex-col gap-4 pb-4 *:drop-shadow-xl"><div class="flex w-full"><div class="p-2 rounded-xl bg-white mx-auto w-120">`);
		PostCard($$renderer2, {
			title: editingData.title === "" ? "<Empty>" : editingData.title,
			slug: editingData.slug,
			excerpt: editingData.excerpt === "" ? "<Empty>" : editingData.excerpt,
			author: {
				name: store_get($$store_subs ??= {}, "$user", user).displayName,
				slug: store_get($$store_subs ??= {}, "$user", user).username
			},
			tags: editingData.tags.split(" ").filter((tag) => tag !== ""),
			src: "/missing.png",
			children: ($$renderer3) => {
				$$renderer3.push(`<div class="absolute full z-20 grid place-items-center border-4 border-dashed border-accent-green rounded-lg opacity-0 hover:opacity-100 hover:scale-105 transition-all duration-100"><span class="stroke-text text-dark bg-text-dark font-bold text-xl drop-shadow-2xl">Change</span></div>`);
			}
		});
		$$renderer2.push(`<!----></div></div> `);
		PostSection($$renderer2, {
			title: editingData.title,
			tags: editingData.tags.split(" ").filter((tag) => tag !== ""),
			date: editingData.date,
			content: renderedText,
			author: editingData.author
		});
		$$renderer2.push(`<!----> <div id="padding" class="svelte-nhvz6o"></div> <div${attr_class("fixed top-full left-1/2 -translate-x-1/2 w-full max-w-400 transition-transform duration-100 -translate-y-14", void 0, { "-translate-y-full": editor.toggled })}><div class="absolute z-9 left-1/2 top-1/2 -translate-1/2 w-[calc(100%+6px)] h-[calc(100%+6px)] bg-dark/20 rounded-t-xl"></div> <div class="relative z-10 flex flex-col items-center bg-white border-2 border-dark not-sm:text-sm rounded-t-xl"><div class="flex justify-between p-2 w-full"><div class="flex gap-2"><div${attr_class("w-25 duo-btn", void 0, {
			"duo-green": true,
			"duo-red": editor.toggled
		})}><button>${escape_html("Expand")}</button></div> `);
		if (mode === "edit") {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<div class="duo-btn duo-blue"><button>Ver. ${escape_html(editor.view === "public" ? "Published" : "Draft")}</button></div>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></div> <div class="flex gap-2">`);
		$$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></div></div> <div class="flex not-lg:flex-col gap-2 w-full h-full p-2 pt-1"><div class="flex grow gap-2"><div class="p-2 space-y-2 w-1/3 bg-primary/40 rounded-lg"><div class="flex not-sm:flex-col"><label class="inline-block min-w-11" for="title">Title:</label> <input id="title" class="grow px-1 min-w-0 bg-white rounded-sm"${attr("value", editingData.title)} autocomplete="off" required/></div> <div class="flex not-sm:flex-col"><label class="inline-block min-w-11" for="slug">Slug:</label> <input id="slug"${attr_class("grow px-1 min-w-0 bg-white rounded-sm", void 0, {
			"bg-red-200!": editingData._slugStatus[editingData.slug] === "used",
			"bg-yellow-200!": editingData._slugStatus[editingData.slug] === "pending",
			"bg-green-200!": editingData._slugStatus[editingData.slug] === "ready"
		})}${attr("value", editingData.slug)} autocomplete="off" required/></div> <div class="flex flex-col"><label class="inline-block" for="slug">Tags:</label> <textarea class="p-1 outline-none bg-white rounded-sm resize-none custom-scrollbar" autocorrect="off" autocomplete="off" rows="2">`);
		const $$body = escape_html(editingData.tags);
		if ($$body) $$renderer2.push(`${$$body}`);
		$$renderer2.push(`</textarea></div> <div class="flex flex-col"><label class="inline-block" for="slug">Excerpt:</label> <textarea class="p-1 outline-none bg-white rounded-sm resize-none custom-scrollbar" autocorrect="off" autocomplete="off" rows="5">`);
		const $$body_1 = escape_html(editingData.excerpt);
		if ($$body_1) $$renderer2.push(`${$$body_1}`);
		$$renderer2.push(`</textarea></div></div> `);
		ContentDebounceEdtior($$renderer2, {
			class: "grow bg-primary/40 p-2 rounded-lg",
			delay: "500",
			onUpdateRendered: (_renderedText) => renderedText = _renderedText,
			onUpdateDraft: (content) => editingData.draft = content,
			disabled: editor.view === "public",
			mediaSyntax,
			mediaDictionary,
			searchMedia,
			forDraft,
			registerForceContent: (fn) => {}
		});
		$$renderer2.push(`<!----></div> <div class="flex gap-2 not-lg:h-40 overflow-hidden">`);
		MediaDictionaryController($$renderer2, { registerSearch: (fn) => searchMedia = fn });
		$$renderer2.push(`<!----></div></div></div></div></article>`);
		if ($$store_subs) unsubscribe_stores($$store_subs);
	});
}

//#endregion
export { PostEditor };
//# sourceMappingURL=PostEditor-ktiFNGcX.js.map