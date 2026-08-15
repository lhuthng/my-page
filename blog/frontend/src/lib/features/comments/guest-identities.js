export const GUEST_IDENTITIES = [
	{ code: 'cutiecube', name: 'Cutie Cube', avatar: '/avatars/cutiecube.webp' },
	{ code: 'straymeowmet', name: 'Stray Meowmet', avatar: '/avatars/straymeowmet.webp' },
	{ code: 'loafphantom', name: 'Loaf Phantom', avatar: '/avatars/loafphantom.webp' }
];

const identityMap = new Map(GUEST_IDENTITIES.map((i) => [i.code, i]));

export function getGuestIdentity(code) {
	return identityMap.get(code) ?? null;
}

export function isValidGuestIdentity(code) {
	return identityMap.has(code);
}
