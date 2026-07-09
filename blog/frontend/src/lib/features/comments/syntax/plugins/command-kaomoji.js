import { SlashCommandHandler } from '../handlers.js';
import { replaceCommandRange } from '../utils.js';

class KaomojiCommandHandler extends SlashCommandHandler {
	constructor({ searchKaomojis, PopoverComponent = null }) {
		super({
			key: 'kaomoji',
			trigger: 'kao',
			search: searchKaomojis,
			meta: {
				loadingLabel: 'Kaomojis',
				emptyText: 'Type a mood to search Kaomojis... (e.g. /kao joy)',
				PopoverComponent
			}
		});
	}

	apply(selection, item) {
		const kaomoji = typeof item === 'string' ? item : item?.value;
		if (!kaomoji) return null;
		const replacement = `@@[${kaomoji}]@@ `;
		const value = replaceCommandRange(selection.value, selection, replacement);
		const next = selection.start + replacement.length;
		return {
			value,
			selectionStart: next,
			selectionEnd: next
		};
	}

	applySuggestion(selection, suggestion) {
		const token = this.buildToken(suggestion);
		const value = replaceCommandRange(selection.value, selection, token);
		return {
			value,
			selectionStart: selection.start + token.length,
			selectionEnd: selection.start + token.length
		};
	}
}

export function createKaomojiCommandPlugin({ searchKaomojis, PopoverComponent = null }) {
	return new KaomojiCommandHandler({ searchKaomojis, PopoverComponent });
}
