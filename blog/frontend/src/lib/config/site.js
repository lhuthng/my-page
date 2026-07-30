export const SITE_ORIGIN = 'https://blog.huuthangle.site';
export const SITE_HOSTNAME = new URL(SITE_ORIGIN).hostname;
export const SITE_NAME = "Huu Thang's Blog";
export const SITE_DESCRIPTION =
	"Thắng's digital garden for software architecture, creative coding, and hands-on experiments.";

export function canonicalUrl(pathname = '/') {
	const url = new URL(pathname, `${SITE_ORIGIN}/`);
	url.search = '';
	url.hash = '';
	return url.href;
}

export function absoluteSiteUrl(value, fallback = '/') {
	if (!value) return canonicalUrl(fallback);

	try {
		return new URL(value).href;
	} catch {
		return new URL(value, `${SITE_ORIGIN}/`).href;
	}
}

export function safeJsonLd(value) {
	return JSON.stringify(value).replaceAll('<', '\\u003c');
}
