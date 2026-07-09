import { GUEST_IDENTITIES } from '../guest-identities.js';

export function normalizeAvatarUrl(url) {
	if (!url) return '/anonymous.gif';
	if (url.startsWith('http://') || url.startsWith('https://')) return url;
	if (url.startsWith('/api/') || url.startsWith('/')) return url;
	return `/api/${String(url).replace(/^\.?\//, '')}`;
}

export function createCommentNodeView(comment) {
	return {
		...comment,
		avatar_url: normalizeAvatarUrl(comment.avatar_url)
	};
}

export function getGuestMeta(code) {
	return GUEST_IDENTITIES.find((identity) => identity.code === code) ?? null;
}

export function createSuggestionPanelState(commandState, mentionState) {
	if (commandState.open) {
		return {
			open: true,
			mode: 'command',
			top: null,
			loading: commandState.loading,
			error: commandState.error,
			selected: commandState.selected,
			items: commandState.items,
			suggestions: commandState.suggestions,
			type: commandState.type
		};
	}

	if (mentionState.open) {
		return {
			open: true,
			mode: 'mention',
			top: null,
			loading: mentionState.loading,
			error: null,
			selected: mentionState.selected,
			items: mentionState.items,
			suggestions: [],
			type: 'mention'
		};
	}

	return {
		open: false,
		mode: null,
		top: null,
		loading: false,
		error: null,
		selected: 0,
		items: [],
		suggestions: [],
		type: null
	};
}
