export const createCommentListState = () => ({
	current: '',
	fetching: false,
	sending: false,
	endReached: false,
	lastId: 0,
	roots: [],
	commentError: ''
});

export const createMentionState = () => ({
	open: false,
	loading: false,
	query: '',
	start: -1,
	selected: 0,
	items: [],
	requestId: 0
});

export const createCommandState = () => ({
	open: false,
	loading: false,
	type: '',
	meta: null,
	query: null,
	start: -1,
	replaceEnd: -1,
	selected: 0,
	items: [],
	suggestions: [],
	error: null,
	requestId: 0
});

export const createDrawerState = () => ({
	showGifSearch: false,
	showKaomojiSearch: false,
	gifQuery: '',
	gifResults: [],
	gifLoading: false,
	gifOffset: 0,
	gifError: null,
	kaomojiMood: '',
	kaomojiResults: [],
	kaomojiSuggestions: [],
	kaomojiLoading: false,
	kaomojiPage: 1,
	kaomojiTotal: 0,
	kaomojiError: null
});

export const createUiState = () => ({
	popoverTop: null,
	showMarkdownHelp: false
});

export const createComposerState = () => ({
	replyTo: null
});

export const createCommentFeatureState = () => ({
	postId: null,
	start: null,
	textarea: null,
	composerSurface: null,
	popoverSurface: null,
	comments: createCommentListState(),
	replyThreads: {},
	composer: createComposerState(),
	mentionState: createMentionState(),
	commandState: createCommandState(),
	drawers: createDrawerState(),
	ui: createUiState()
});

export const createEmptyState = createCommentFeatureState;

export function resetObject(target, next) {
	for (const key of Object.keys(target)) {
		delete target[key];
	}
	Object.assign(target, next);
}
