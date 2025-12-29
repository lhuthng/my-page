//#region .svelte-kit/adapter-bun/chunks/common.js
function preventDefault(e) {
	e.preventDefault();
}
const mediaSyntax = /\@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]/g;
function textToDate(text) {
	const dt = new Date(text.split(" ")[0]);
	const options = {
		year: "numeric",
		month: "short",
		day: "numeric"
	};
	return dt.toLocaleDateString("en-US", options);
}
function nowToDate() {
	const dt = /* @__PURE__ */ new Date();
	const options = {
		year: "numeric",
		month: "short",
		day: "numeric"
	};
	return dt.toLocaleDateString("en-US", options);
}

//#endregion
export { mediaSyntax, nowToDate, preventDefault, textToDate };
//# sourceMappingURL=common-BLrvVRDf.js.map