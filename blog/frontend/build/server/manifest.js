//#region .svelte-kit/adapter-bun/manifest.js
const manifest = (() => {
	function __memo(fn) {
		let value;
		return () => value ??= value = fn();
	}
	return {
		appDir: "_app",
		appPath: "_app",
		assets: new Set([
			"favicon.ico",
			"logo.png",
			"logo.svg",
			"me-1.jpg",
			"missing.png",
			"robots.txt",
			"thinkcats.jpg",
			"underconstruction.jpg"
		]),
		mimeTypes: {
			".png": "image/png",
			".svg": "image/svg+xml",
			".jpg": "image/jpeg",
			".txt": "text/plain"
		},
		_: {
			client: {
				start: "_app/immutable/entry/start.BN8-GeZx.js",
				app: "_app/immutable/entry/app.CVkj70yt.js",
				imports: [
					"_app/immutable/entry/start.BN8-GeZx.js",
					"_app/immutable/chunks/BM5FkKPH.js",
					"_app/immutable/chunks/C9-y2tij.js",
					"_app/immutable/chunks/BYr3L0YT.js",
					"_app/immutable/chunks/BQNcAQLV.js",
					"_app/immutable/entry/app.CVkj70yt.js",
					"_app/immutable/chunks/C9-y2tij.js",
					"_app/immutable/chunks/MxVqpocq.js",
					"_app/immutable/chunks/d6JjwGM_.js",
					"_app/immutable/chunks/BYr3L0YT.js",
					"_app/immutable/chunks/BtnrPDDH.js",
					"_app/immutable/chunks/Dud9yQQh.js",
					"_app/immutable/chunks/B1sCrlU5.js",
					"_app/immutable/chunks/BodVPAaS.js",
					"_app/immutable/chunks/DeJMVDUL.js",
					"_app/immutable/chunks/DVIB-bf-.js"
				],
				stylesheets: [],
				fonts: [],
				uses_env_dynamic_public: false
			},
			nodes: [
				__memo(() => import("./chunks/0-C2aWuTkV.js")),
				__memo(() => import("./chunks/1-BEgAKu4_.js")),
				__memo(() => import("./chunks/2-ZMG9p2Sq.js")),
				__memo(() => import("./chunks/3-DPZL-YfM.js")),
				__memo(() => import("./chunks/4-CgAkyB01.js")),
				__memo(() => import("./chunks/5-Bzdef8KI.js")),
				__memo(() => import("./chunks/6-BM_4AzAU.js")),
				__memo(() => import("./chunks/7-DADiGvyN.js")),
				__memo(() => import("./chunks/8-BMgQX7ZQ.js")),
				__memo(() => import("./chunks/9-CRME5rV_.js")),
				__memo(() => import("./chunks/10-pHsEZofR.js")),
				__memo(() => import("./chunks/11-CidTJIdD.js")),
				__memo(() => import("./chunks/12-C8oIHy50.js")),
				__memo(() => import("./chunks/13-CV5VeMT6.js")),
				__memo(() => import("./chunks/14-BLth5605.js")),
				__memo(() => import("./chunks/15-KYQbmbzA.js")),
				__memo(() => import("./chunks/16-D4ieCJZL.js")),
				__memo(() => import("./chunks/17-CIMGpm6B.js")),
				__memo(() => import("./chunks/18-B5KQFHLj.js")),
				__memo(() => import("./chunks/19-Urk3Dqlt.js")),
				__memo(() => import("./chunks/20-BDWfxWXm.js")),
				__memo(() => import("./chunks/21-DSBt1PKW.js"))
			],
			remotes: {},
			routes: [
				{
					id: "/",
					pattern: /^\/$/,
					params: [],
					page: {
						layouts: [0],
						errors: [1],
						leaf: 7
					},
					endpoint: null
				},
				{
					id: "/about",
					pattern: /^\/about\/?$/,
					params: [],
					page: {
						layouts: [0],
						errors: [1],
						leaf: 8
					},
					endpoint: null
				},
				{
					id: "/api/auth/login",
					pattern: /^\/api\/auth\/login\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-WuyLfmqZ.js"))
				},
				{
					id: "/api/auth/logout",
					pattern: /^\/api\/auth\/logout\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-hB6p6E7t.js"))
				},
				{
					id: "/api/media",
					pattern: /^\/api\/media\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-iRqJlOzg.js"))
				},
				{
					id: "/api/media/s/[slug]",
					pattern: /^\/api\/media\/s\/([^/]+?)\/?$/,
					params: [{
						"name": "slug",
						"optional": false,
						"rest": false,
						"chained": false
					}],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-_BgzcJAT.js"))
				},
				{
					id: "/api/posts",
					pattern: /^\/api\/posts\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-DUUgaTUR.js"))
				},
				{
					id: "/api/posts/id/[id]/comments",
					pattern: /^\/api\/posts\/id\/([^/]+?)\/comments\/?$/,
					params: [{
						"name": "id",
						"optional": false,
						"rest": false,
						"chained": false
					}],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-CG0C7AJC.js"))
				},
				{
					id: "/api/posts/latest",
					pattern: /^\/api\/posts\/latest\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-CLSWqLX5.js"))
				},
				{
					id: "/api/posts/s/[slug]",
					pattern: /^\/api\/posts\/s\/([^/]+?)\/?$/,
					params: [{
						"name": "slug",
						"optional": false,
						"rest": false,
						"chained": false
					}],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-B2Xm2Mu0.js"))
				},
				{
					id: "/api/users",
					pattern: /^\/api\/users\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-CQNnRuM3.js"))
				},
				{
					id: "/api/users/me",
					pattern: /^\/api\/users\/me\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-BTtFBB0Z.js"))
				},
				{
					id: "/api/users/[slug]/posts",
					pattern: /^\/api\/users\/([^/]+?)\/posts\/?$/,
					params: [{
						"name": "slug",
						"optional": false,
						"rest": false,
						"chained": false
					}],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-CzkX_EnN.js"))
				},
				{
					id: "/api/[...path]",
					pattern: /^\/api(?:\/([^]*))?\/?$/,
					params: [{
						"name": "path",
						"optional": false,
						"rest": true,
						"chained": true
					}],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-Cbe9QA8n.js"))
				},
				{
					id: "/dashboard",
					pattern: /^\/dashboard\/?$/,
					params: [],
					page: {
						layouts: [0, 2],
						errors: [1, 3],
						leaf: 9
					},
					endpoint: null
				},
				{
					id: "/dashboard/media/manager",
					pattern: /^\/dashboard\/media\/manager\/?$/,
					params: [],
					page: {
						layouts: [0, 2],
						errors: [1, 3],
						leaf: 10
					},
					endpoint: null
				},
				{
					id: "/dashboard/posts",
					pattern: /^\/dashboard\/posts\/?$/,
					params: [],
					page: {
						layouts: [0, 2],
						errors: [1, 3],
						leaf: 11
					},
					endpoint: null
				},
				{
					id: "/dashboard/posts/id/[post_id]",
					pattern: /^\/dashboard\/posts\/id\/([^/]+?)\/?$/,
					params: [{
						"name": "post_id",
						"optional": false,
						"rest": false,
						"chained": false
					}],
					page: {
						layouts: [
							0,
							2,
							,
						],
						errors: [
							1,
							3,
							4
						],
						leaf: 12
					},
					endpoint: null
				},
				{
					id: "/dashboard/posts/new",
					pattern: /^\/dashboard\/posts\/new\/?$/,
					params: [],
					page: {
						layouts: [0, 2],
						errors: [1, 3],
						leaf: 13
					},
					endpoint: null
				},
				{
					id: "/login",
					pattern: /^\/login\/?$/,
					params: [],
					page: {
						layouts: [0, 5],
						errors: [1, ,],
						leaf: 14
					},
					endpoint: null
				},
				{
					id: "/posts",
					pattern: /^\/posts\/?$/,
					params: [],
					page: {
						layouts: [0],
						errors: [1],
						leaf: 15
					},
					endpoint: null
				},
				{
					id: "/posts/[slug]",
					pattern: /^\/posts\/([^/]+?)\/?$/,
					params: [{
						"name": "slug",
						"optional": false,
						"rest": false,
						"chained": false
					}],
					page: {
						layouts: [0],
						errors: [1],
						leaf: 16
					},
					endpoint: null
				},
				{
					id: "/profiles/[slug]",
					pattern: /^\/profiles\/([^/]+?)\/?$/,
					params: [{
						"name": "slug",
						"optional": false,
						"rest": false,
						"chained": false
					}],
					page: {
						layouts: [0, ,],
						errors: [1, 6],
						leaf: 17
					},
					endpoint: null
				},
				{
					id: "/projects",
					pattern: /^\/projects\/?$/,
					params: [],
					page: {
						layouts: [0],
						errors: [1],
						leaf: 18
					},
					endpoint: null
				},
				{
					id: "/tags",
					pattern: /^\/tags\/?$/,
					params: [],
					page: {
						layouts: [0],
						errors: [1],
						leaf: 19
					},
					endpoint: null
				},
				{
					id: "/tags/[slug]",
					pattern: /^\/tags\/([^/]+?)\/?$/,
					params: [{
						"name": "slug",
						"optional": false,
						"rest": false,
						"chained": false
					}],
					page: {
						layouts: [0],
						errors: [1],
						leaf: 20
					},
					endpoint: null
				},
				{
					id: "/users/[slug]",
					pattern: /^\/users\/([^/]+?)\/?$/,
					params: [{
						"name": "slug",
						"optional": false,
						"rest": false,
						"chained": false
					}],
					page: {
						layouts: [0],
						errors: [1],
						leaf: 21
					},
					endpoint: null
				}
			],
			prerendered_routes: new Set([]),
			matchers: async () => {
				return {};
			},
			server_assets: {}
		}
	};
})();
const prerendered = new Set([]);
const base = "";

//#endregion
export { base, manifest, prerendered };
//# sourceMappingURL=manifest.js.map