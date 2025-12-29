export function load(event) {
	const { user, accessToken } = event.locals;
	return {
		user,
		accessToken,
	};
}
