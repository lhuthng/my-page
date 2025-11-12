import { user } from "$lib/client/user.js";
import { proxyFallback, route } from "$lib/server/proxy.js";

export async function POST({ request, fetch }) {
	let res = await proxyFallback({
		request,
		params: {
			path: "auth/login",
		},
	});

	if (!res.ok) {
		const text = await res.text();
		return new Response(text, { status: res.status });
	}
	const setCookieHeader = res.headers.get("Set-Cookie");

	const { token, token_type } = await res.json();

	res = await fetch(route("user/me"), {
		method: "GET",
		headers: {
			Authorization: `${token_type} ${token}`,
			"Content-Type": "application/json",
		},
	});

	if (!res.ok) {
		const text = await res.text();
		return new Response(text, { status: res.status });
	}

	const { display_name, role } = await res.json();

	const headers = new Headers({
		"Content-Type": "application/json",
	});
	headers.set("Set-Cookie", setCookieHeader);

	return new Response(
		JSON.stringify({
			display_name,
			role,
			token,
			token_type,
		}),
		{ status: 200, headers },
	);
}
