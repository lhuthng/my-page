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
			"DEAD_LINK.png",
			"favicon.ico",
			"html/l-game/favicon.ico",
			"html/l-game/html5game/L Game.js",
			"html/l-game/html5game/L Game_texture_0.png",
			"html/l-game/html5game/L Game_texture_1.png",
			"html/l-game/html5game/snd_cell.mp3",
			"html/l-game/html5game/snd_cell.ogg",
			"html/l-game/html5game/snd_coin_collect.mp3",
			"html/l-game/html5game/snd_coin_collect.ogg",
			"html/l-game/html5game/snd_coin_drop.mp3",
			"html/l-game/html5game/snd_coin_drop.ogg",
			"html/l-game/html5game/snd_coin_jump.mp3",
			"html/l-game/html5game/snd_coin_jump.ogg",
			"html/l-game/html5game/snd_extra.mp3",
			"html/l-game/html5game/snd_extra.ogg",
			"html/l-game/html5game/snd_jump_.mp3",
			"html/l-game/html5game/snd_jump_.ogg",
			"html/l-game/html5game/snd_loop.mp3",
			"html/l-game/html5game/snd_loop.ogg",
			"html/l-game/html5game/snd_lose.mp3",
			"html/l-game/html5game/snd_lose.ogg",
			"html/l-game/html5game/snd_piece.mp3",
			"html/l-game/html5game/snd_piece.ogg",
			"html/l-game/html5game/snd_start.mp3",
			"html/l-game/html5game/snd_start.ogg",
			"html/l-game/html5game/snd_swap.mp3",
			"html/l-game/html5game/snd_swap.ogg",
			"html/l-game/html5game/snd_tick.mp3",
			"html/l-game/html5game/snd_tick.ogg",
			"html/l-game/html5game/snd_win.mp3",
			"html/l-game/html5game/snd_win.ogg",
			"html/l-game/html5game/sound/worklets/audio-worklet.js",
			"html/l-game/html5game/splash.png",
			"html/l-game/index.html",
			"html/l-game/l-game-thumbnail.png",
			"html/l-game/options.ini",
			"logo.png",
			"logo.svg",
			"me-1.jpg",
			"missing.png",
			"models/demo.glb",
			"robots.txt",
			"thinkcats.jpg",
			"underconstruction.jpg"
		]),
		mimeTypes: {
			".png": "image/png",
			".js": "text/javascript",
			".mp3": "audio/mpeg",
			".ogg": "audio/ogg",
			".html": "text/html",
			".ini": "text/plain",
			".svg": "image/svg+xml",
			".jpg": "image/jpeg",
			".glb": "model/gltf-binary",
			".txt": "text/plain"
		},
		_: {
			client: {
				start: "_app/immutable/entry/start.BsVKu-Sv.js",
				app: "_app/immutable/entry/app.CLF86aXt.js",
				imports: [
					"_app/immutable/entry/start.BsVKu-Sv.js",
					"_app/immutable/chunks/CFIEO4tZ.js",
					"_app/immutable/chunks/CH8riAWf.js",
					"_app/immutable/chunks/D_VySCWd.js",
					"_app/immutable/chunks/C8lBvSGv.js",
					"_app/immutable/chunks/QOyedSr_.js",
					"_app/immutable/chunks/BGWKUAbB.js",
					"_app/immutable/entry/app.CLF86aXt.js",
					"_app/immutable/chunks/D_VySCWd.js",
					"_app/immutable/chunks/C8lBvSGv.js",
					"_app/immutable/chunks/DsnmJJEf.js",
					"_app/immutable/chunks/CH8riAWf.js",
					"_app/immutable/chunks/QOyedSr_.js",
					"_app/immutable/chunks/DKFBgVa7.js",
					"_app/immutable/chunks/tULUmgGa.js",
					"_app/immutable/chunks/Cei6ewVj.js",
					"_app/immutable/chunks/D3Yk_FSP.js"
				],
				stylesheets: [],
				fonts: [],
				uses_env_dynamic_public: false
			},
			nodes: [
				__memo(() => import("./chunks/0-D8de-e2l.js")),
				__memo(() => import("./chunks/1-DlqRiCqV.js")),
				__memo(() => import("./chunks/2-CA1RA8ZH.js")),
				__memo(() => import("./chunks/3-y14ueD0h.js")),
				__memo(() => import("./chunks/4-Duxyn0SH.js")),
				__memo(() => import("./chunks/5-CCnmILkl.js")),
				__memo(() => import("./chunks/6-BOrsoi-K.js")),
				__memo(() => import("./chunks/7-C7ZIt-N-.js")),
				__memo(() => import("./chunks/8-D1lWNtCL.js")),
				__memo(() => import("./chunks/9-BzUWcmN6.js")),
				__memo(() => import("./chunks/10-DAZGCxxd.js")),
				__memo(() => import("./chunks/11-sxuoIqCH.js")),
				__memo(() => import("./chunks/12-CGdUtqI8.js")),
				__memo(() => import("./chunks/13-DZCmJ_nI.js")),
				__memo(() => import("./chunks/14-CPufr47E.js")),
				__memo(() => import("./chunks/15-C2cdXZxK.js")),
				__memo(() => import("./chunks/16-BC9ue9kR.js")),
				__memo(() => import("./chunks/17-CL18K-yv.js")),
				__memo(() => import("./chunks/18-DY-aNil8.js")),
				__memo(() => import("./chunks/19-CeIEjh4n.js")),
				__memo(() => import("./chunks/20-DOeW1VRe.js")),
				__memo(() => import("./chunks/21-3VxMokAK.js")),
				__memo(() => import("./chunks/22-C-O4WUjT.js"))
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
					endpoint: __memo(() => import("./chunks/_server-NLrFrD6A.js"))
				},
				{
					id: "/api/auth/logout",
					pattern: /^\/api\/auth\/logout\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-weqSeqMg.js"))
				},
				{
					id: "/api/media",
					pattern: /^\/api\/media\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-CsncbcIu.js"))
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
					endpoint: __memo(() => import("./chunks/_server-CpuNB2p9.js"))
				},
				{
					id: "/api/posts",
					pattern: /^\/api\/posts\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-CQdUnAkF.js"))
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
					endpoint: __memo(() => import("./chunks/_server-DBSVj3iA.js"))
				},
				{
					id: "/api/posts/latest",
					pattern: /^\/api\/posts\/latest\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-Di7OoZUt.js"))
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
					endpoint: __memo(() => import("./chunks/_server-CwAXbdze.js"))
				},
				{
					id: "/api/users",
					pattern: /^\/api\/users\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-B17D2p2s.js"))
				},
				{
					id: "/api/users/me",
					pattern: /^\/api\/users\/me\/?$/,
					params: [],
					page: null,
					endpoint: __memo(() => import("./chunks/_server-DtGbKhnZ.js"))
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
					endpoint: __memo(() => import("./chunks/_server-tiELl2JD.js"))
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
					endpoint: __memo(() => import("./chunks/_server-B6yvzuW_.js"))
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
					id: "/projects/l-game",
					pattern: /^\/projects\/l-game\/?$/,
					params: [],
					page: {
						layouts: [0],
						errors: [1],
						leaf: 19
					},
					endpoint: null
				},
				{
					id: "/series",
					pattern: /^\/series\/?$/,
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