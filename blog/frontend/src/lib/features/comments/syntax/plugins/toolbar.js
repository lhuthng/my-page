import { insertText, wrapText } from '../utils.js';

export function createToolbarActions() {
	return {
		insertHeader(value, selectionStart, selectionEnd) {
			return insertText(value, selectionStart, selectionEnd, '# ');
		},
		bold(value, selectionStart, selectionEnd) {
			return wrapText(value, selectionStart, selectionEnd, '**');
		},
		italic(value, selectionStart, selectionEnd) {
			return wrapText(value, selectionStart, selectionEnd, '_');
		},
		code(value, selectionStart, selectionEnd) {
			return wrapText(value, selectionStart, selectionEnd, '`');
		},
		insertText
	};
}
