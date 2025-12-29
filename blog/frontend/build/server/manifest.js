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
				start: "_app/immutable/entry/start.DtaPnr6f.js",
				app: "_app/immutable/entry/app.AOnfBtZp.js",
				imports: [
					"_app/immutable/entry/start.DtaPnr6f.js",
					"_app/immutable/chunks/B5X_EVLB.js",
					"_app/immutable/chunks/ChRVst1r.js",
					"_app/immutable/chunks/DIeogL5L.js",
					"_app/immutable/chunks/BRm8bsAD.js",
					"_app/immutable/chunks/DXBmiNFF.js",
					"_app/immutable/chunks/Dnjdv7Ru.js",
					"_app/immutable/entry/app.AOnfBtZp.js",
					"_app/immutable/chunks/ChRVst1r.js",
					"_app/immutable/chunks/DIeogL5L.js",
					"_app/immutable/chunks/CKHu4Vf7.js",
					"_app/immutable/chunks/keZNrdrU.js",
					"_app/immutable/chunks/DsnmJJEf.js",
					"_app/immutable/chunks/Dnjdv7Ru.js",
					"_app/immutable/chunks/DfhCrWJZ.js",
					"_app/immutable/chunks/su8FdElh.js",
					"_app/immutable/chunks/BZa1XKl2.js",
					"_app/immutable/chunks/apr4qJEX.js",
					"_app/immutable/chunks/C9CqtO5Y.js",
					"_app/immutable/chunks/DGag4Teo.js",
					"_app/immutable/chunks/BRm8bsAD.js"
				],
				stylesheets: [],
				fonts: [],
				uses_env_dynamic_public: false
			},
			nodes: [
				__memo(() => import("./chunks/0-BWAt7ecw.js")),
				__memo(() => import("./chunks/1-BUzNXR3t.js")),
				__memo(() => import("./chunks/2-CChGm_Uo.js")),
				__memo(() => import("./chunks/3-B2f6andZ.js")),
				__memo(() => import("./chunks/4-Dn94e-uX.js")),
				__memo(() => import("./chunks/5-C7rmW9zo.js")),
				__memo(() => import("./chunks/6-Cros4PVM.js")),
				__memo(() => import("./chunks/7-l-304y22.js")),
				__memo(() => import("./chunks/8-DqET6UgY.js")),
				__memo(() => import("./chunks/9-a-VHlSuC.js")),
				__memo(() => import("./chunks/10-Cx58TXkL.js")),
				__memo(() => import("./chunks/11-A1Es3r3k.js")),
				__memo(() => import("./chunks/12-CXua9i7B.js")),
				__memo(() => import("./chunks/13-h1ESWvMv.js")),
				__memo(() => import("./chunks/14-CbPh8YrB.js")),
				__memo(() => import("./chunks/15-Beki_0bs.js")),
				__memo(() => import("./chunks/16-DZ2aQ6Fh.js")),
				__memo(() => import("./chunks/17-DIo6n7ti.js")),
				__memo(() => import("./chunks/18-Co1YWGd3.js")),
				__memo(() => import("./chunks/19-BO4IpsLT.js")),
				__memo(() => import("./chunks/20-cD2gtVwt.js")),
				__memo(() => import("./chunks/21-PURbvE9M.js")),
				__memo(() => import("./chunks/22-C_Yk135F.js")),
				__memo(() => import("./chunks/23-jpiB8xhT.js")),
				__memo(() => import("./chunks/24-DZ3pL09X.js"))
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
						leaf: 8
					},
					endpoint: null
				},
				{
					id: "/api/auth/login",
					pattern: /^\/api\/auth\/login\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-B7AI8Pwn.js"))
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
					endpoint: __memo(() => import("./chunks/_server-CwksYEEV.js"))
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
					endpoint: __memo(() => import("./chunks/_server-D5wO9jEr.js"))
				},
				{
					id: "/api/posts",
					pattern: /^\/api\/posts\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-C3b43Fns.js"))
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
					endpoint: __memo(() => import("./chunks/_server-DLzPiXYa.js"))
				},
				{
					id: "/api/posts/latest",
					pattern: /^\/api\/posts\/latest\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-COA5Mi5I.js"))
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
					endpoint: __memo(() => import("./chunks/_server-do-j6bRl.js"))
				},
				{
					id: "/api/users",
					pattern: /^\/api\/users\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-rQf96XZr.js"))
				},
				{
					id: "/api/users/me",
					pattern: /^\/api\/users\/me\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-BXzJfE15.js"))
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
					endpoint: __memo(() => import("./chunks/_server-CBM-tQdV.js"))
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
					endpoint: __memo(() => import("./chunks/_server-CoG11o1k.js"))
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
					id: "/editor-demo",
					pattern: /^\/editor-demo\/?$/,
					params: [],
					page: {
						layouts: [0],
						errors: [1],
						leaf: 14
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
						leaf: 15
					},
					endpoint: null
				},
				{
					id: "/posts",
					pattern: /^\/posts\/?$/,
					params: [],
					page: {
						layouts: [0, 6],
						errors: [1, ,],
						leaf: 16
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
						layouts: [0, 6],
						errors: [1, ,],
						leaf: 17
					},
					endpoint: null
				},
				{
					id: "/profiles",
					pattern: /^\/profiles\/?$/,
					params: [],
					page: {
						layouts: [0],
						errors: [1],
						leaf: 18
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
						errors: [1, 7],
						leaf: 19
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
						leaf: 20
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
						leaf: 21
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
						leaf: 22
					},
					endpoint: null
				},
				{
					id: "/users",
					pattern: /^\/users\/?$/,
					params: [],
					page: {
						layouts: [0],
						errors: [1],
						leaf: 23
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
						leaf: 24
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