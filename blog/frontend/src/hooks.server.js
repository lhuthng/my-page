import { dev } from '$app/environment';
import { env } from '$env/dynamic/private';
import { SITE_HOSTNAME, SITE_ORIGIN } from '$lib/config/site.js';
import { fixClientRoute, route } from '$lib/server/proxy';

const SAFE_METHODS = new Set(['GET', 'HEAD', 'OPTIONS']);
const NOINDEX_PATHS = [
	'/api',
	'/dashboard',
	'/login',
	'/privacy',
	'/reset-password',
	'/verify-email'
];

function commaSeparated(value) {
	return (value ?? '')
		.split(',')
		.map((item) => item.trim())
		.filter(Boolean);
}

function allowedHosts() {
	return new Set([
		SITE_HOSTNAME,
		'localhost',
		'127.0.0.1',
		'::1',
		...(env.FLY_APP_NAME ? [`${env.FLY_APP_NAME}.fly.dev`] : []),
		...commaSeparated(env.ALLOWED_HOSTS)
	]);
}

function requestHostname(event) {
	const host = event.request.headers.get('host');
	if (!host) return event.url.hostname;

	try {
		return new URL(`http://${host}`).hostname;
	} catch {
		return '';
	}
}

function trustedOrigins(event) {
	return new Set([
		SITE_ORIGIN,
		event.url.origin,
		'https://portfolio.huuthangle.site',
		'http://localhost:3000',
		'http://localhost:3004',
		'http://localhost:5000',
		...commaSeparated(env.TRUSTED_ORIGINS)
	]);
}

function isNoindexPath(pathname) {
	return NOINDEX_PATHS.some((prefix) => pathname === prefix || pathname.startsWith(`${prefix}/`));
}

function hardenHtml(html) {
	const officialOrigin = SITE_ORIGIN.replaceAll('$', '$$$$');
	const absoluteRootReferences = html.replace(
		/\b(href|src|action|poster)="\/(?!\/)/g,
		`$1="${officialOrigin}/`
	);

	const redirectGuard = `<script>(()=>{const h=location.hostname;if(h!=='${SITE_HOSTNAME}'&&h!=='localhost'&&h!=='127.0.0.1'&&h!=='::1'){location.replace('${SITE_ORIGIN}'+location.pathname+location.search+location.hash)}})();</script>`;
	return absoluteRootReferences.replace('</head>', `${redirectGuard}</head>`);
}

function applySecurityHeaders(response, event, noindex) {
	// Responses returned by the API proxy can carry immutable Undici headers.
	// Always copy them into a new Response before applying site-wide headers.
	const mutableResponse = new Response(response.body, {
		status: response.status,
		statusText: response.statusText,
		headers: new Headers(response.headers)
	});

	mutableResponse.headers.set('Content-Security-Policy', "frame-ancestors 'self'");
	mutableResponse.headers.set('X-Frame-Options', 'SAMEORIGIN');
	mutableResponse.headers.set('X-Content-Type-Options', 'nosniff');
	mutableResponse.headers.set('Referrer-Policy', 'strict-origin-when-cross-origin');
	mutableResponse.headers.set('Permissions-Policy', 'camera=(), microphone=(), geolocation=()');

	if (!dev && event.url.protocol === 'https:') {
		mutableResponse.headers.set('Strict-Transport-Security', 'max-age=31536000; includeSubDomains');
	}

	if (noindex) {
		mutableResponse.headers.set('X-Robots-Tag', 'noindex, nofollow, noarchive');
		mutableResponse.headers.set('Cache-Control', 'private, no-store');
	}

	return mutableResponse;
}

async function populateUser(event) {
	const refreshToken = event.cookies.get('refresh-token');

	event.locals.accessToken = null;
	event.locals.user = null;
	event.locals.role = null;

	if (!refreshToken) return;

	let res = await event.fetch(route('auth/refresh'), {
		method: 'POST',
		headers: {
			cookie: `refresh-token=${refreshToken}`
		}
	});

	if (!res.ok) {
		event.cookies.delete('refresh-token', {
			path: '/',
			httpOnly: true,
			secure: true,
			sameSite: 'lax'
		});
		return;
	}

	const { token_type: type, token } = await res.json();

	res = await event.fetch(route('users/me'), {
		method: 'GET',
		headers: {
			Authorization: `${type} ${token}`,
			'Content-Type': 'application/json'
		}
	});

	if (!res.ok) return;

	const { username, display_name: displayName, role, avatar_url: avatarUrl } = await res.json();

	event.locals.accessToken = { type, token };
	event.locals.user = {
		username,
		displayName,
		role,
		avatarUrl: fixClientRoute(avatarUrl)
	};
	event.locals.role = role;
}

export async function handle({ event, resolve }) {
	if (!dev && !allowedHosts().has(requestHostname(event))) {
		return applySecurityHeaders(new Response('Misdirected request', { status: 421 }), event, true);
	}

	const requestOrigin = event.request.headers.get('origin');
	if (
		!SAFE_METHODS.has(event.request.method) &&
		requestOrigin &&
		!trustedOrigins(event).has(requestOrigin)
	) {
		return applySecurityHeaders(
			new Response('Cross-origin request rejected', { status: 403 }),
			event,
			true
		);
	}

	const accept = event.request.headers.get('accept') ?? '';
	if (accept.includes('text/html')) {
		event
			.fetch(route('analytics/visit'), {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					...(event.request.headers.get('cf-ipcountry')
						? { 'CF-IPCountry': event.request.headers.get('cf-ipcountry') }
						: {})
				},
				body: JSON.stringify({ path: event.url.pathname })
			})
			.catch((error) => {
				console.warn('[analytics] failed to track visit:', error);
			});

		await populateUser(event);
	}

	const noindex = isNoindexPath(event.url.pathname);
	const response = await resolve(event, {
		transformPageChunk: !dev ? ({ html }) => hardenHtml(html) : undefined
	});

	return applySecurityHeaders(response, event, noindex || response.status >= 400);
}
