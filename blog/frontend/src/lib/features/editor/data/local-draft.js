/**
 * Local-only autosave of the draft body + metadata, so a crashed tab or an
 * accidental close doesn't lose unsaved edits. Never touches the server —
 * that's `save-controller`'s job — so it can't interact with the optimistic
 * lock.
 */

const PREFIX = 'editor-draft:';

function keyFor(kind, id) {
	return `${PREFIX}${kind}:${id ?? 'new'}`;
}

/**
 * @param {'post'|'project'} kind
 * @param {string|number|''} id
 * @param {object} snapshot plain-serializable editor fields worth recovering
 */
export function saveLocalDraft(kind, id, snapshot) {
	try {
		localStorage.setItem(keyFor(kind, id), JSON.stringify({ ...snapshot, savedAt: Date.now() }));
	} catch {
		// localStorage can throw (quota exceeded, private browsing). Losing the
		// safety net silently beats crashing the editor over it.
	}
}

/**
 * @param {'post'|'project'} kind
 * @param {string|number|''} id
 * @returns {object | null}
 */
export function loadLocalDraft(kind, id) {
	try {
		const raw = localStorage.getItem(keyFor(kind, id));
		return raw ? JSON.parse(raw) : null;
	} catch {
		return null;
	}
}

/**
 * @param {'post'|'project'} kind
 * @param {string|number|''} id
 */
export function clearLocalDraft(kind, id) {
	try {
		localStorage.removeItem(keyFor(kind, id));
	} catch {
		// ignore
	}
}
