import "./utils2-B3oc7zZR.js";
import "./internal-CyqLiTQC.js";
import { writable } from "./utils-B7r1aIrD.js";
import "./clsx-BYt6phfV.js";
import { escape_html } from "./escaping-DoGxUxJF.js";
import "./context-BKhPkoFN.js";
import { attr, attr_class, attr_style, attributes, ensure_array_like, store_get, store_mutate, unsubscribe_stores } from "./chunks-CQcQZdVb.js";
import "./state.svelte-CeWcm6KF.js";
import { auth } from "./user-CPizBtTY.js";
import { preventDefault } from "./common-BLrvVRDf.js";

//#region .svelte-kit/adapter-bun/entries/pages/dashboard/media/manager/_page.svelte.js
function MediaDirectory($$renderer, $$props) {
	let { cellWidth, cellHeight, onclick, children, $$slots, $$events,...rest } = $$props;
	$$renderer.push(`<div${attributes({ ...rest })}><div class="full grid"${attr_style("", {
		"grid-template-columns": `repeat(auto-fit,minmax(${cellWidth},1fr))`,
		"grid-template-rows": `repeat(auto-fit,${cellHeight})`
	})}>`);
	children?.($$renderer);
	$$renderer.push(`<!----></div></div>`);
}
const detailsCache = writable({});
function MediaEditForm($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		var $$store_subs;
		let { shortName } = $$props;
		let details = store_get($$store_subs ??= {}, "$detailsCache", detailsCache)[shortName];
		$$renderer2.push(`<div class="flex flex-col gap-2">`);
		if (!details) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<span>Search and select any media to edit the file</span>`);
		} else {
			$$renderer2.push("<!--[!-->");
			if (details?.status === "waiting") {
				$$renderer2.push("<!--[-->");
				$$renderer2.push(`<span>Waiting, I'm loading</span>`);
			} else {
				$$renderer2.push("<!--[!-->");
				$$renderer2.push("<!--[!-->");
				$$renderer2.push(`<!--]-->`);
			}
			$$renderer2.push(`<!--]-->`);
		}
		$$renderer2.push(`<!--]--></div>`);
		if ($$store_subs) unsubscribe_stores($$store_subs);
	});
}
function MediumEntity($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		let { size, file, isSelected, ok, onclick, ondblclick } = $$props;
		$$renderer2.push(`<button${attr_class(`flex flex-col h-fit text-center rounded-sm gap-2 mx-auto p-2 cursor-pointer ${isSelected ? "bg-gray-100" : ""}`)}${attr_style("", { "max-width": `${size + 20}px` })}><div class="flex items-center justify-center rounded-sm drop-shadow-2xl"${attr_style("", {
			width: `${size}px`,
			height: `${size}px`
		})}><div class="flex"${attr_style("", {
			height: `${size}px`,
			width: `${size}px`
		})}><div class="m-auto relative"><img class="m-auto object-contain rounded-sm overflow-hidden"${attr("src", file.url)}${attr("alt", file.name)}${attr_style("", {
			"max-height": `${size}px`,
			"max-width": `${size}px`
		})}/> `);
		if (ok === true) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<svg class="absolute right-1 bottom-1 w-4 h-4" width="0px" height="0px" viewBox="0 0 48 48" xmlns="http://www.w3.org/2000/svg"><circle fill="#4CAF50" cx="24" cy="24" r="21"></circle><g fill="none" stroke="#FFFFFF" stroke-width="4"><line x1="15.4" y1="22.6" x2="21" y2="28.2"></line><line x1="21" y1="28.2" x2="34.6" y2="14.6"></line></g></svg>`);
		} else {
			$$renderer2.push("<!--[!-->");
			if (ok === false) {
				$$renderer2.push("<!--[-->");
				$$renderer2.push(`<svg class="absolute right-1 bottom-1 w-4 h-4" width="0px" height="0px" viewBox="0 0 48 48" xmlns="http://www.w3.org/2000/svg"><circle fill="#F44336" cx="24" cy="24" r="21"></circle><g fill="none" stroke="#FFFFFF" stroke-width="4"><line x1="15" y1="15" x2="33" y2="33"></line><line x1="33" y1="15" x2="15" y2="33"></line></g></svg>`);
			} else $$renderer2.push("<!--[!-->");
			$$renderer2.push(`<!--]-->`);
		}
		$$renderer2.push(`<!--]--></div></div></div> <span${attr_class(`wrap-break-word line-clamp-2 ${isSelected ? "select-text" : "select-none"}`)}>${escape_html(file.name)}</span></button>`);
	});
}
function Portal_1($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		let { target, children, $$slots, $$events,...rest } = $$props;
		$$renderer2.push(`<div${attributes({ ...rest })}>`);
		children?.($$renderer2);
		$$renderer2.push(`<!----></div>`);
	});
}
function MediaEditor($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		var $$store_subs;
		let { keyword, detailPanel } = $$props;
		let requestCache = {};
		let selection = void 0;
		let deKeyword = keyword;
		$$renderer2.push(`<div class="relative full">`);
		MediaDirectory($$renderer2, {
			class: "full p-2",
			cellWidth: "120px",
			cellHeight: "200px",
			onclick: () => {},
			children: ($$renderer3) => {
				if (requestCache[deKeyword]?.status === "success") {
					$$renderer3.push("<!--[-->");
					$$renderer3.push(`<!--[-->`);
					const each_array = ensure_array_like(requestCache[deKeyword]?.results);
					for (let index = 0, $$length = each_array.length; index < $$length; index++) {
						let item = each_array[index];
						MediumEntity($$renderer3, {
							size: 80,
							file: {
								name: item.short_name,
								url: item.url
							},
							isSelected: selection === item.short_name,
							onclick: async () => {
								if (!store_get($$store_subs ??= {}, "$detailsCache", detailsCache)[item.short_name]) {
									const details = { status: "waiting" };
									store_mutate($$store_subs ??= {}, "$detailsCache", detailsCache, store_get($$store_subs ??= {}, "$detailsCache", detailsCache)[item.short_name] = { ...details });
									const res = await fetch(`/api/media/d/${item.short_name}`, {
										method: "GET",
										headers: { Authorization: auth() }
									});
									if (res.ok) {
										details.status = "success";
										details.result = await res.json();
									} else details.status = "failed";
									store_mutate($$store_subs ??= {}, "$detailsCache", detailsCache, store_get($$store_subs ??= {}, "$detailsCache", detailsCache)[item.short_name] = { ...details });
								}
								selection = item.short_name;
							},
							ondblclick: () => {}
						});
					}
					$$renderer3.push(`<!--]-->`);
				} else $$renderer3.push("<!--[!-->");
				$$renderer3.push(`<!--]-->`);
			},
			$$slots: { default: true }
		});
		$$renderer2.push(`<!----></div> `);
		Portal_1($$renderer2, {
			class: "p-2",
			target: detailPanel,
			children: ($$renderer3) => {
				MediaEditForm($$renderer3, { shortName: selection });
			},
			$$slots: { default: true }
		});
		$$renderer2.push(`<!---->`);
		if ($$store_subs) unsubscribe_stores($$store_subs);
	});
}
function MediaUploaderForm($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		let { media } = $$props;
		let draft = {};
		$$renderer2.push(`<div class="flex flex-col gap-2">`);
		if (!media) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<span>?</span>`);
		} else {
			$$renderer2.push("<!--[!-->");
			$$renderer2.push(`<span class="text-center">Details</span> <form class="flex flex-col gap-2" method="post"><label class="ml-auto mr-4 text-sm" for="file-type">${escape_html(draft.type)}</label> <fieldset class="border-2 rounded-lg pt-1 pb-2 px-2"><legend class="font-semibold text-xs left-2 px-1" for="short-name">Short name</legend> <input${attr("disabled", media?.ok, true)} class="w-full focus:outline-0" type="text"${attr("value", draft.shortName)} name="short-name"/></fieldset> <fieldset class="border-2 rounded-lg pt-1 pb-2 px-2"><legend class="font-semibold text-xs left-2 px-1" for="description">Description</legend> <textarea${attr("disabled", media?.ok, true)} class="w-full focus:outline-0 resize-none" type="text" rows="4" name="short-name">`);
			const $$body = escape_html(draft.description);
			if ($$body) $$renderer2.push(`${$$body}`);
			$$renderer2.push(`</textarea></fieldset> <button${attr("disabled", media?.ok, true)} class="ml-auto w-fit border-2 px-1 rounded-lg cursor-pointer" type="submit">Submit</button></form>`);
		}
		$$renderer2.push(`<!--]--></div>`);
	});
}
function MediaUploader($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		let { detailPanel } = $$props;
		let media = [];
		let selection = void 0;
		function appendMedia(files) {
			for (let index = 0; index < files.length; index++) {
				const file = files[index];
				if (file && file.type.startsWith("image/")) {
					let name = file.name;
					let medium = {
						name,
						type: file.type,
						url: URL.createObjectURL(file),
						file
					};
					media.push(medium);
				}
			}
		}
		function handleDrop(e) {
			e.preventDefault();
			appendMedia(e.dataTransfer.files);
		}
		$$renderer2.push(`<div class="relative full" role="listitem">`);
		if (media.length === 0) {
			$$renderer2.push("<!--[-->");
			$$renderer2.push(`<div class="absolute full p-2 pointer-events-none" role="listitem"><div class="flex full justify-center items-center border-2 border-dashed rounded-sm"><p>Drop a media here</p></div></div>`);
		} else $$renderer2.push("<!--[!-->");
		$$renderer2.push(`<!--]--> `);
		MediaDirectory($$renderer2, {
			class: "full p-2",
			cellWidth: "120px",
			cellHeight: "200px",
			ondrop: handleDrop,
			ondragover: preventDefault,
			onclick: (e) => {
				e.target === e.currentTarget && (selection = void 0);
			},
			children: ($$renderer3) => {
				$$renderer3.push(`<!--[-->`);
				const each_array = ensure_array_like(media);
				for (let index = 0, $$length = each_array.length; index < $$length; index++) {
					let medium = each_array[index];
					MediumEntity($$renderer3, {
						size: 80,
						file: medium,
						onclick: () => selection = index,
						isSelected: index === selection,
						ok: medium.ok
					});
				}
				$$renderer3.push(`<!--]-->`);
			},
			$$slots: { default: true }
		});
		$$renderer2.push(`<!----></div> `);
		Portal_1($$renderer2, {
			class: "p-2",
			target: detailPanel,
			children: ($$renderer3) => {
				MediaUploaderForm($$renderer3, { media: media[selection] });
			},
			$$slots: { default: true }
		});
		$$renderer2.push(`<!---->`);
	});
}
function MediaManager($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		let { editMode, changeMode, $$slots, $$events,...rest } = $$props;
		let keyword = "";
		let detailPanel = void 0;
		$$renderer2.push(`<div${attributes({ ...rest })}><div class="flex flex-col px-2 not-md:pr-6 py-4 gap-4"><div class="flex justify-between"><div class="flex gap-2"><input${attr("disabled", editMode !== true, true)} class="rounded-xl ring-1 px-2 disabled:bg-gray-100 disabled:text-gray-400 disabled:cursor-not-allowed" type="text" placeholder="keywords"${attr("value", keyword)}/> <button${attr("disabled", editMode !== true, true)} class="border-black rounded-full px-2 ring-1 hover:bg-gray-50 disabled:bg-gray-100 disabled:text-gray-400 disabled:cursor-not-allowed" type="button">Search</button></div> <button class="cursor-pointer" type="button">${escape_html(editMode ? "Upload" : "Edit")}</button></div> <div class="relative z-9"><div class="relative flex rounded-lg ring-1 h-180 overflow-hidden z-10"><div class="grow bg-blue-100 min-h-full">`);
		if (editMode) {
			$$renderer2.push("<!--[-->");
			MediaEditor($$renderer2, {
				detailPanel,
				keyword
			});
		} else {
			$$renderer2.push("<!--[!-->");
			MediaUploader($$renderer2, { detailPanel });
			$$renderer2.push(`<!---->/>`);
		}
		$$renderer2.push(`<!--]--></div> <div class="relative h-full bg-white transition-all duration-200"${attr_style("", { width: "0" })}><div class="absolute w-80 h-full"><div></div></div></div></div> <div class="absolute top-2 left-full z-9"><button${attr_class(`bg-blue-100 rounded-r-lg ring-1 px-2 pl-1 cursor-pointer transition-all duration-200 -translate-x-1 hover:-translate-x-0.5 hover:bg-blue-200`)}${attr_style("", { "writing-mode": "vertical-lr" })}>Details</button></div></div></div></div>`);
	});
}
function _page($$renderer) {
	let editMode = true;
	MediaManager($$renderer, {
		class: "w-cap",
		editMode,
		changeMode: () => editMode = !editMode
	});
}

//#endregion
export { _page as default };
//# sourceMappingURL=_page.svelte-kcfunUiE.js.map