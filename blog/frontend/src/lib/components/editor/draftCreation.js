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
	goto(gotoPath(editor.createdEntryId));
}
