import { resizeTextarea } from '$lib/dom/auto-resize';

export function createComposerController({ state, getTextarea, engine, onAfterInput }) {
	function syncLayout(selectionStart = null, selectionEnd = selectionStart, focus = true) {
		requestAnimationFrame(() => {
			const textarea = getTextarea();
			if (!textarea) return;

			resizeTextarea(textarea);
			if (focus) {
				textarea.focus();
			}
			if (selectionStart != null) {
				textarea.setSelectionRange(selectionStart, selectionEnd ?? selectionStart);
			}
			onAfterInput?.();
		});
	}

	function applySelectionChange(result) {
		if (!result) return;
		state.comments.current = result.value;
		syncLayout(result.selectionStart, result.selectionEnd);
	}

	function currentSelection() {
		const textarea = getTextarea();
		const value = state.comments.current;
		return {
			value,
			selectionStart: textarea?.selectionStart ?? value.length,
			selectionEnd: textarea?.selectionEnd ?? value.length
		};
	}

	return {
		syncLayout,
		applySelectionChange,
		insertAtCursor(text) {
			const selection = currentSelection();
			applySelectionChange(
				engine.toolbar.insertText(
					selection.value,
					selection.selectionStart,
					selection.selectionEnd,
					text
				)
			);
		},
		insertHeader() {
			const selection = currentSelection();
			applySelectionChange(
				engine.toolbar.insertHeader(
					selection.value,
					selection.selectionStart,
					selection.selectionEnd
				)
			);
		},
		wrapBold() {
			const selection = currentSelection();
			applySelectionChange(
				engine.toolbar.bold(selection.value, selection.selectionStart, selection.selectionEnd)
			);
		},
		wrapItalic() {
			const selection = currentSelection();
			applySelectionChange(
				engine.toolbar.italic(selection.value, selection.selectionStart, selection.selectionEnd)
			);
		},
		wrapCode() {
			const selection = currentSelection();
			applySelectionChange(
				engine.toolbar.code(selection.value, selection.selectionStart, selection.selectionEnd)
			);
		}
	};
}
