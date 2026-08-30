export const SITE_ORIGIN = 'https://huuthangle.site';
export const SITE_HOSTNAME = new URL(SITE_ORIGIN).hostname;
export const SITE_NAME = "Huu Thang's Blog";
export const SITE_DESCRIPTION =
	"Thắng's digital garden for software architecture, creative coding, and hands-on experiments.";

export const SITE_AUTHOR = {
	name: 'Huu Thang Le',
	alternateName: 'Thắng',
	url: `${SITE_ORIGIN}/about`,
	image: `${SITE_ORIGIN}/thinkcats.jpg`,
	sameAs: [
		'https://github.com/lhuthng',
		'https://www.linkedin.com/in/huuthangle/',
		'https://portfolio.huuthangle.site',
		'https://www.youtube.com/@memofie',
		'https://www.artstation.com/lhuthng'
	]
};

export const SITE_OG_IMAGE = `${SITE_ORIGIN}/thinkcats.jpg`;
export const SITE_OG_IMAGE_WIDTH = 1676;
export const SITE_OG_IMAGE_HEIGHT = 878;
export const SITE_LOCALE = 'en_US';

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
