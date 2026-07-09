import { SlashCommandHandler } from '../handlers.js';
import { replaceCommandRange } from '../utils.js';

export function buildGifMarkdown(gif) {
	const gifUrl = gif?.images?.original?.url;
	if (!gifUrl) return null;

	const safeTitle = String(gif?.title ?? 'gif')
		.replace(/\[/g, ' ')
		.replace(/\]/g, ' ')
		.trim();

	return `![${safeTitle || 'gif'}](${gifUrl})`;
}

class GifCommandHandler extends SlashCommandHandler {
	constructor({ searchGifs, PopoverComponent = null }) {
		super({
			key: 'gif',
			trigger: 'gif',
			search: searchGifs,
			meta: {
				loadingLabel: 'GIFs',
				emptyText: 'Type to search GIFs... (e.g. /gif cats)',
				PopoverComponent
			}
		});
	}

	apply(selection, item) {
		const markdownImage = buildGifMarkdown(item);
		if (!markdownImage) return null;
		const replacement = `${markdownImage} `;
		const value = replaceCommandRange(selection.value, selection, replacement);
		const next = selection.start + replacement.length;
		return {
			value,
			selectionStart: next,
			selectionEnd: next
		};
	}
}

export function createGifCommandPlugin({ searchGifs, PopoverComponent = null }) {
	return new GifCommandHandler({ searchGifs, PopoverComponent });
}
