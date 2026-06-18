export function pick(array) {
	return array[(Math.random() * array.length) >> 0];
}

export const el = $state({ pbody: undefined, mbody: undefined });
