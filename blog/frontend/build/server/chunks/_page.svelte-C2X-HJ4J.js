import "./utils2-B3oc7zZR.js";
import "./internal-CyqLiTQC.js";
import "./utils-B7r1aIrD.js";
import "./clsx-BYt6phfV.js";
import "./escaping-DoGxUxJF.js";
import "./context-BKhPkoFN.js";
import "./chunks-CQcQZdVb.js";
import "./state.svelte-CeWcm6KF.js";
import "./user-CPizBtTY.js";
import "./SearchButton-BdwpD_3M.js";
import "./PostCard-EnmsPyMN.js";
import "./common-BLrvVRDf.js";
import "./index2-DUFc1Uun.js";
import "./html-CocCBXUl.js";
import "./PostSection-DxZK4VZX.js";
import { PostEditor } from "./PostEditor-ktiFNGcX.js";

//#region .svelte-kit/adapter-bun/entries/pages/dashboard/posts/id/_post_id_/_page.svelte.js
function _page($$renderer, $$props) {
	const { data } = $$props;
	const { id, title, slug, content, draft, tags, excerpt, medium_short_names: mediumShortNames, medium_urls: mediumUrls } = data;
	PostEditor($$renderer, {
		mode: "edit",
		data: {
			id,
			title,
			slug,
			content,
			draft,
			tags,
			excerpt,
			mediumShortNames,
			mediumUrls
		}
	});
}

//#endregion
export { _page as default };
//# sourceMappingURL=_page.svelte-C2X-HJ4J.js.map