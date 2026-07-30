export async function POST({ cookies }) {
	cookies.delete('refresh-token', {
		path: '/',
		httpOnly: true,
		secure: true,
		sameSite: 'lax'
	});
	return new Response(null, { status: 204 });
}
