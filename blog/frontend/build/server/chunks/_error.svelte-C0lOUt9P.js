import "./clsx-BYt6phfV.js";
import { escape_html } from "./escaping-DoGxUxJF.js";

//#region .svelte-kit/adapter-bun/entries/pages/dashboard/_error.svelte.js
function _error($$renderer, $$props) {
	let { code, status } = $$props;
	$$renderer.push(`<div>${escape_html(code)}
    ${escape_html(status)}</div>`);
}

//#endregion
export { _error as default };
//# sourceMappingURL=_error.svelte-C0lOUt9P.js.map