export function openCreatedDraftPrompt(editor, createdEntryId) {
	editor.createdEntryId = createdEntryId;
	editor.createPromptOpen = true;
	editor.createPromptBusy = false;
	editor.createPromptError = '';
}

export async function finishCreatedDraftPrompt({
	editor,
	publishNow,
	publishPath,
	gotoPath,
	fetchImpl,
	authHeader,
	goto
}) {
	if (!editor.createdEntryId) return;
	editor.createPromptError = '';
	if (publishNow) {
		editor.createPromptBusy = true;
		const publishRes = await fetchImpl(publishPath(editor.createdEntryId), {
			method: 'POST',
			headers: { Authorization: authHeader }
		});
		editor.createPromptBusy = false;
		if (!publishRes.ok) {
			editor.createPromptError = await publishRes.text();
			return;
		}
	}
	editor.createPromptOpen = false;
	// Callers that guard against in-progress navigation (the editor's
	// unsaved-changes prompt) need this to actually finish — including running
	// any `beforeNavigate` hooks — before they lift the guard. `goto` without
	// `await` returns before that happens, which let the guard's own
	// `beforeNavigate` callback fire on this exact navigation and cancel it.
	await goto(gotoPath(editor.createdEntryId));
}
