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

export function isAllowedEmbed(value) {
	try {
		return FRAME_ORIGINS.includes(new URL(value).origin);
	} catch {
		return false;
	}
}
