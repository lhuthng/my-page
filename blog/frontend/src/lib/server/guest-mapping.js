const map = new Map();

export function setGuestMapping(commentId, guestIdentity) {
	map.set(commentId, guestIdentity);
}

export function getGuestMapping(commentId) {
	return map.get(commentId) ?? null;
}
