// What the editor's status pill says.
//
// The pill is a summary of the *save lifecycle*, not of the dirty flag alone.
// It used to read `isDirty ? 'Unsaved' : 'Saved'`, which had no way to express
// "a save is in flight" (it kept saying Unsaved while the request was in the
// air), a failure, or a publish. Pure on purpose, so the state machine is
// testable from plain Node like the rest of `model/`.

/**
 * @param {object} args
 * @param {'create'|'edit'} args.mode
 * @param {'idle'|'saving'|'saved'|'error'|'conflict'} args.status
 * @param {boolean} args.isDirty
 * @param {boolean} [args.isPublishing]
 * @returns {{label: string, tone: 'busy'|'dirty'|'error'|'saved', title: string} | null}
 *   `null` when there is nothing to report: a fresh create form has not saved
 *   anything yet, so claiming "Saved" would be a lie.
 */
export function saveIndicator({ mode, status, isDirty, isPublishing = false }) {
	// Ordered by what the user needs to know first: work in flight beats a
	// dirty flag (the flag is already true during a save, which is exactly what
	// made the old pill read "Unsaved" while saving).
	if (isPublishing) {
		return { label: 'Publishing…', tone: 'busy', title: 'Publishing is in flight' };
	}
	if (status === 'saving') {
		return { label: 'Saving…', tone: 'busy', title: 'Saving is in flight' };
	}
	if (status === 'conflict') {
		return { label: 'Conflict', tone: 'error', title: 'Someone else saved this first' };
	}
	if (status === 'error') {
		return { label: 'Save failed', tone: 'error', title: 'The last save failed' };
	}
	if (isDirty) {
		return { label: 'Unsaved', tone: 'dirty', title: 'Unsaved changes' };
	}
	if (mode === 'create') return null;
	return { label: 'Saved', tone: 'saved', title: 'No unsaved changes' };
}
