import { auth } from '$lib/auth/user.svelte.js';

class ApiError extends Error {
	constructor(message, status, data) {
		super(message);
		this.name = 'ApiError';
		this.status = status;
		this.data = data;
	}
}

async function request(path, { method = 'GET', body, auth: useAuth = true, ...opts } = {}) {
	const headers = { ...opts.headers };
	if (useAuth) {
		headers.Authorization = auth();
	}
	if (body && !(body instanceof FormData)) {
		headers['Content-Type'] = 'application/json';
		body = JSON.stringify(body);
	}
	const res = await fetch(`/api/${path}`, { method, headers, body, ...opts });
	if (!res.ok) {
		const text = await res.text();
		let data;
		try {
			data = JSON.parse(text);
		} catch {
			data = text;
		}
		throw new ApiError(data?.message || text || res.statusText, res.status, data);
	}
	if (res.status === 204) return null;
	return res.json();
}

export const api = {
	get: (path, opts) => request(path, { ...opts, method: 'GET' }),
	post: (path, opts) => request(path, { ...opts, method: 'POST' }),
	patch: (path, opts) => request(path, { ...opts, method: 'PATCH' }),
	put: (path, opts) => request(path, { ...opts, method: 'PUT' }),
	delete: (path, opts) => request(path, { ...opts, method: 'DELETE' })
};

export { ApiError };
