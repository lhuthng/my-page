// Origins the site is willing to frame. Shared by the CSP `frame-src`
// directive (svelte.config.js) and the markdown `:::iframe` plugin, so the
// policy and the renderer cannot drift apart.
export const FRAME_ORIGINS = [
	'https://www.youtube.com',
	'https://www.youtube-nocookie.com',
	'https://challenges.cloudflare.com',
	'https://synrecordia.site',
	'https://synrecordia.netlify.app'
];

// The html5 demos are served by the backend itself (project-demos/...), and
// production sets BACKEND_ORIGIN, so those demo iframes point at the backend
// host - a different origin than the site even though both are first-party.
// `frame-src` must name it or every html5 demo is blocked. Unlike
// FRAME_ORIGINS this is not part of the markdown `:::iframe` allowlist:
// posts have no reason to embed the backend directly.
export const DEMO_FRAME_ORIGINS = ['https://api.huuthangle.site'];

export function isAllowedEmbed(value) {
	try {
		return FRAME_ORIGINS.includes(new URL(value).origin);
	} catch {
		return false;
	}
}
