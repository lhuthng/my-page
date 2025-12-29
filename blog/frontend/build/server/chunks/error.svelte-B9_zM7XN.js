import "./utils2-B3oc7zZR.js";
import "./internal-CyqLiTQC.js";
import "./utils-B7r1aIrD.js";
import "./clsx-BYt6phfV.js";
import { escape_html } from "./escaping-DoGxUxJF.js";
import { getContext } from "./context-BKhPkoFN.js";
import "./state.svelte-CeWcm6KF.js";
import "./client-C-eUpif6.js";

//#region .svelte-kit/adapter-bun/entries/fallbacks/error.svelte.js
function context() {
	return getContext("__request__");
}
const page$1 = {
	get error() {
		return context().page.error;
	},
	get status() {
		return context().page.status;
	}
};
const page = page$1;
function Error$1($$renderer, $$props) {
	$$renderer.component(($$renderer2) => {
		$$renderer2.push(`<h1>${escape_html(page.status)}</h1> <p>${escape_html(page.error?.message)}</p>`);
	});
}

//#endregion
export { Error$1 as default };
//# sourceMappingURL=error.svelte-B9_zM7XN.js.map