export function replaceRange(value, start, end, replacement) {
	return `${value.slice(0, start)}${replacement}${value.slice(end)}`;
}

export function replaceCommandRange(value, context, replacement) {
	if (!context) return value;
	return replaceRange(value, context.start, context.replaceEnd, replacement);
}

export function insertText(value, selectionStart, selectionEnd, text) {
	return {
		value: `${value.slice(0, selectionStart)}${text}${value.slice(selectionEnd)}`,
		selectionStart: selectionStart + text.length,
		selectionEnd: selectionStart + text.length
	};
}

export function wrapText(value, selectionStart, selectionEnd, prefix, suffix = prefix) {
	return {
		value:
			value.slice(0, selectionStart) +
			prefix +
			value.slice(selectionStart, selectionEnd) +
			suffix +
			value.slice(selectionEnd),
		selectionStart: selectionStart + prefix.length,
		selectionEnd: selectionEnd + prefix.length
	};
}
