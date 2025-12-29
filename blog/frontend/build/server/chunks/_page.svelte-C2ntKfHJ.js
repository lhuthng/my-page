import "./utils2-B3oc7zZR.js";
import "./clsx-BYt6phfV.js";
import { escape_html } from "./escaping-DoGxUxJF.js";
import "./context-BKhPkoFN.js";
import { attr, attr_class, attr_style, bind_props, clsx$1 as clsx, ensure_array_like } from "./chunks-CQcQZdVb.js";
import { mediaWithShortcutPlugin } from "./index2-DUFc1Uun.js";
import { html } from "./html-CocCBXUl.js";
import MarkdownIt from "markdown-it/dist/index.cjs.js";

//#region .svelte-kit/adapter-bun/entries/pages/editor-demo/_page.svelte.js
function ImageEntity($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		let { index, length, name, isRenaming, dictionary, handlers } = $$props;
		let newName = name;
		let isValid = handlers?.checkName(index, newName);
		$$renderer2.push(`<div class="flex w-full h-full gap-2 bg-gray-100 rounded-sm p-1"><span class="my-auto">${escape_html(index + 1)}.</span> <img class="w-20 h-20 rounded-sm ring-1 object-contain"${attr("src", dictionary[name])}${attr("alt", name)}/> <div class="flex flex-col-reverse justify-between grow p-1"><div class="flex gap-4"><button class="text-blue-500 cursor-pointer hover:brightness-140"${attr_style("", { "text-decoration": isRenaming ? "underline" : "none" })}>edit</button> <button class="text-blue-500 cursor-pointer hover:brightness-140">delete</button> `);
		if (isRenaming) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<button class="text-blue-500 cursor-pointer hover:brightness-140">ok</button>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--></div> <div class="flex flex-col grow justify-center font-mono">`);
		if (isRenaming) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<textarea${attr_class(`w-full resize-none rounded-sm overflow-hidden ${isValid ? "bg-green-200" : "bg-red-100"}`)} rows="2">`);
			const $$body = escape_html(newName);
			if ($$body) $$renderer2.push(`${$$body}`);
			$$renderer2.push(`</textarea>`);
		} else {
			$$renderer2.push("<!--[!-->");
			$$renderer2.push(`<span${attr_style("", { "word-break": "break-all" })}>${escape_html(name)}</span>`);
		}
		$$renderer2.push(`<!--]--></div></div> <div class="flex flex-col justify-between w-4 h-full [&amp;>svg]:transition-transform [&amp;>svg]:duration-100 [&amp;>svg]:origin-center"><svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true"${attr_class(clsx(index > 0 ? "cursor-pointer hover:scale-110" : "opacity-30"))}><polyline points="4,10 8,6 12,10" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"></polyline></svg> <svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true"${attr_class(clsx(index < length - 1 ? "cursor-pointer hover:scale-110" : "opacity-30"))}><polyline points="4,6 8,10 12,6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"></polyline></svg></div></div>`);
	});
}
function ImageManager($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		let { mediaDictionary = void 0 } = $$props;
		let imageList = [];
		let renamingIndex = void 0;
		const entityHandlers = {
			remove(index) {
				const newDict = { ...mediaDictionary };
				const name = imageList[index];
				const url = newDict[name];
				URL.revokeObjectURL(url);
				imageList.splice(index, 1);
				delete newDict[name];
				mediaDictionary = { ...newDict };
			},
			upward(index) {
				if (index <= 0) return;
				[imageList[index], imageList[index - 1]] = [imageList[index - 1], imageList[index]];
			},
			downward(index) {
				if (index >= imageList.length - 1) return;
				[imageList[index], imageList[index + 1]] = [imageList[index + 1], imageList[index]];
			},
			checkName(index, name) {
				return !(name.trim() in mediaDictionary) || imageList[index] === name.trim();
			},
			startRenaming(index) {
				renamingIndex = index;
				return true;
			},
			stopRenaming(index) {
				renamingIndex = void 0;
			},
			submitRename(index, newName) {
				if (this.checkName(index, newName)) {
					const oldName = imageList[index];
					const image = mediaDictionary[oldName];
					const newDict = { ...mediaDictionary };
					delete newDict[oldName];
					imageList[index] = newName;
					newDict[newName] = image;
					if (renamingIndex === index) renamingIndex = void 0;
					mediaDictionary = { ...newDict };
				}
			}
		};
		$$renderer2.push(`<p>Your image(s):</p> <div class="w-full border-2 border-dashed border-gray-400 grid grid-cols-[repeat(auto-fit,minmax(20rem,1fr))] gap-2 items-center text-gray-500 bg-gray-200 rounded-lg p-1" role="list">`);
		if (imageList.length === 0) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<div class="m-auto text-center"><p>Drop an image (PNG, WEBP, JPG...) here</p> <p class="text-sm text-gray-400 my-1">or</p></div>`);
		} else {
			$$renderer2.push("<!--[!-->");
			$$renderer2.push(`<!--[-->`);
			const each_array = ensure_array_like(imageList);
			for (let index = 0, $$length = each_array.length; index < $$length; index++) {
				let name = each_array[index];
				ImageEntity($$renderer2, {
					index,
					name,
					isRenaming: index === renamingIndex,
					length: imageList.length,
					dictionary: mediaDictionary,
					handlers: entityHandlers
				});
			}
			$$renderer2.push(`<!--]-->`);
		}
		$$renderer2.push(`<!--]--></div>`);
		bind_props($$props, { mediaDictionary });
	});
}
function MarkdownRenderer($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		const { mediaDictionary } = $$props;
		let md = (() => new MarkdownIt().use(mediaWithShortcutPlugin, { mediaDictionary }))();
		let markdown = "";
		let renderedMarkdown = md.render(markdown);
		$$renderer2.push(`<div class="flex"><textarea class="bg-red-100">`);
		const $$body = escape_html(markdown);
		if ($$body) $$renderer2.push(`${$$body}`);
		$$renderer2.push(`</textarea> <div class="rendered-markdown grow bg-blue-100">${html(renderedMarkdown)}</div></div>`);
	});
}
function _page($$renderer) {
	let mediaDictionary = {};
	let $$settled = true;
	let $$inner_renderer;
	function $$render_inner($$renderer2) {
		ImageManager($$renderer2, {
			get mediaDictionary() {
				return mediaDictionary;
			},
			set mediaDictionary($$value) {
				mediaDictionary = $$value;
				$$settled = false;
			}
		});
		$$renderer2.push(`<!----> `);
		MarkdownRenderer($$renderer2, { mediaDictionary });
		$$renderer2.push(`<!---->`);
	}
	do {
		$$settled = true;
		$$inner_renderer = $$renderer.copy();
		$$render_inner($$inner_renderer);
	} while (!$$settled);
	$$renderer.subsume($$inner_renderer);
}

//#endregion
export { _page as default };
//# sourceMappingURL=_page.svelte-C2ntKfHJ.js.map