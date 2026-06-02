export const createCommentCache = () => {
	const values = new Map();
	const inFlight = new Map();

	return {
		inFlight,
		has: (key) => values.has(key),
		get: (key) => values.get(key),
		set: (key, value) => values.set(key, value),
		delete: (key) => values.delete(key),
		clear: () => {
			values.clear();
			inFlight.clear();
		}
	};
};
