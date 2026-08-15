/**
 * Thin transport layer for the post/project editor. Payload construction
 * (what goes into the `post_data`/`project_data` JSON blob, which files get
 * attached) stays with the view-model, since that differs materially between
 * kinds — this only knows the URL shapes and response parsing.
 */

const PLURAL = { post: 'posts', project: 'projects' };

/**
 * @param {'post'|'project'} kind
 * @param {string} slug
 * @param {typeof fetch} fetchImpl
 */
export async function checkSlugAvailable(kind, slug, fetchImpl = fetch) {
	const res = await fetchImpl(`/api/${PLURAL[kind]}/check?slug=${encodeURIComponent(slug)}`, {
		method: 'GET',
		headers: { 'Content-Type': 'application/json' }
	});
	if (!res.ok) throw new Error(await res.text());
	const { exists } = await res.json();
	return !exists;
}

/**
 * @param {'post'|'project'} kind
 * @param {FormData} formData
 * @param {string} authHeader
 * @param {typeof fetch} fetchImpl
 * @returns {Promise<{id: number}>}
 */
export async function createEntry(kind, formData, authHeader, fetchImpl = fetch) {
	const res = await fetchImpl(`/api/${PLURAL[kind]}/new`, {
		method: 'POST',
		headers: { Authorization: authHeader },
		body: formData
	});
	if (!res.ok) throw new Error(await res.text());
	return res.json();
}

/**
 * @param {'post'|'project'} kind
 * @param {number|string} id
 * @param {FormData} formData
 * @param {string} authHeader
 * @param {typeof fetch} fetchImpl
 * @returns {Promise<{updated_at?: string}>}
 */
export async function patchEntry(kind, id, formData, authHeader, fetchImpl = fetch) {
	const res = await fetchImpl(`/api/${PLURAL[kind]}/id/${id}`, {
		method: 'PATCH',
		headers: { Authorization: authHeader },
		body: formData
	});
	if (res.status === 409) {
		const currentUpdatedAt = await res.text();
		const err = new Error('conflict');
		err.conflict = true;
		err.currentUpdatedAt = currentUpdatedAt;
		throw err;
	}
	if (!res.ok) throw new Error(await res.text());
	// Both endpoints return `{}` when nothing changed and no body when the
	// content type doesn't warrant one; tolerate either.
	const text = await res.text();
	if (!text) return {};
	try {
		return JSON.parse(text);
	} catch {
		return {};
	}
}

/**
 * @param {'post'|'project'} kind
 * @param {number|string} id
 * @param {string} authHeader
 * @param {typeof fetch} fetchImpl
 */
export async function publishEntry(kind, id, authHeader, fetchImpl = fetch) {
	const res = await fetchImpl(`/api/${PLURAL[kind]}/id/${id}`, {
		method: 'POST',
		headers: { Authorization: authHeader }
	});
	if (!res.ok) throw new Error(await res.text());
}

/**
 * @param {number|string} id
 * @param {string} authHeader
 * @param {typeof fetch} fetchImpl
 */
export async function deleteProjectDraft(id, authHeader, fetchImpl = fetch) {
	await fetchImpl(`/api/projects/id/${id}`, {
		method: 'DELETE',
		headers: { Authorization: authHeader }
	}).catch(() => {});
}
