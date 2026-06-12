function sanitizeRedirectTarget(rawRedirectTo) {
	if (!rawRedirectTo || !rawRedirectTo.startsWith('/')) return '/';
	if (rawRedirectTo.startsWith('//')) return '/';

	return rawRedirectTo;
}

export async function load({ url, setHeaders }) {
	const register = url.searchParams.get('register');
	const redirectTo = sanitizeRedirectTarget(url.searchParams.get('redirectTo'));
	setHeaders({
		'cache-control': 'public, max-age=60, s-maxage=60'
	});
	return { register, redirectTo };
}
