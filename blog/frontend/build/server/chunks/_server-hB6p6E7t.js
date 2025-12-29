//#region .svelte-kit/adapter-bun/entries/endpoints/api/auth/logout/_server.js
async function POST({ cookies }) {
	cookies.delete("refresh-token", {
		path: "/",
		httpOnly: true,
		secure: true,
		sameSite: "strict"
	});
	return new Response(null, { status: 204 });
}

//#endregion
export { POST };
//# sourceMappingURL=_server-hB6p6E7t.js.map