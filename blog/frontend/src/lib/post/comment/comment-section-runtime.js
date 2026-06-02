import { resizeTextarea } from '$lib/client/auto-resize';
import { auth, user } from '$lib/client/user';
import { gsap } from 'gsap';
import MarkdownIt from 'markdown-it';
import { get } from 'svelte/store';
import { codeHighlightPlugin, mentionProfilePlugin, kaomojiPlugin } from '$lib/custom-rules';
import { createCommentCache } from './comment-cache';
import {
	buildCommandToken,
	COMMENT_COMMANDS,
	getActiveCommentCommand,
	replaceCommandRange
} from './comment-syntax';

const rootLimit = 3;
const replyLimit = 5;
const mentionMinChars = 3;
const mentionMaxProfiles = 5;
const mentionRegex = /(^|[\s(>])@([A-Za-z0-9_-]{3,32})\b/g;
const mentionDebounceMs = 250;
const commandDebounceMs = 250;
const commandMaxKaomojis = 12;
const commandMaxGifs = 12;

const createEmptyComments = () => ({
	current: '',
	fetching: false,
	sending: false,
	endReached: false,
	lastId: 0,
	roots: []
});

const createEmptyMentionState = () => ({
	open: false,
	loading: false,
	query: '',
	start: -1,
	selected: 0,
	items: [],
	requestId: 0
});

const createEmptyCommandState = () => ({
	open: false,
	loading: false,
	type: '',
	query: null,
	start: -1,
	replaceEnd: -1,
	hasClosingParen: false,
	selected: 0,
	items: [],
	suggestions: [],
	error: null,
	requestId: 0
});

const createEmptyState = () => ({
	postId: null,
	start: null,
	textarea: null,
	composerSurface: null,
	comments: createEmptyComments(),
	replyThreads: {},
	replyTo: null,
	mentionState: createEmptyMentionState(),
	commandState: createEmptyCommandState(),
	popoverTop: null,
	showGifSearch: false,
	showKaomojiSearch: false,
	showMarkdownHelp: false,
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

const resetObject = (target, next) => {
	for (const key of Object.keys(target)) {
		delete target[key];
	}
	Object.assign(target, next);
};

export const createCommentSectionRuntime = (state, getUserAvatarUrl) => {
	const mentionDictionary = {};
	const mentionProfileCache = new Map();
	const mentionProfileInFlight = new Map();
	const mentionSearchCache = createCommentCache();
	const mentionSearchInFlight = mentionSearchCache.inFlight;
	const rootPageCache = createCommentCache();
	const rootPageInFlight = rootPageCache.inFlight;
	const replyPageCache = createCommentCache();
	const replyPageInFlight = replyPageCache.inFlight;
	const commandKaomojiCache = createCommentCache();
	const commandKaomojiInFlight = commandKaomojiCache.inFlight;
	const commandGifCache = createCommentCache();
	const commandGifInFlight = commandGifCache.inFlight;
	const drawerGifCache = createCommentCache();
	const drawerGifInFlight = drawerGifCache.inFlight;
	const drawerKaomojiCache = createCommentCache();
	const drawerKaomojiInFlight = drawerKaomojiCache.inFlight;

	const md = new MarkdownIt()
		.use(codeHighlightPlugin)
		.use(mentionProfilePlugin, { mentionDictionary })
		.use(kaomojiPlugin);

	let mentionDebounceTimer;
	let commandDebounceTimer;
	let popoverAnchorRaf;

	const comments = state.comments;
	const replyThreads = state.replyThreads;
	const mentionState = state.mentionState;
	const commandState = state.commandState;

	const getPostId = () => state.postId;
	const getTextarea = () => state.textarea;
	const getComposerSurface = () => state.composerSurface;
	const getStart = () => state.start;

	const normalizeAvatarUrl = (url) => {
		if (!url) return '/anonymous.gif';
		if (url.startsWith('http://') || url.startsWith('https://')) return url;
		if (url.startsWith('/api/') || url.startsWith('/')) return url;
		return `/api/${url.replace(/^\.?\//, '')}`;
	};

	const extractMentionUsernames = (value) => {
		if (!value) return [];
		const usernames = new Set();
		let match;
		const scan = new RegExp(mentionRegex.source, 'g');
		while ((match = scan.exec(value)) !== null) {
			if (match[2]) usernames.add(match[2]);
		}
		return [...usernames];
	};

	const ensureMentionProfile = async (username) => {
		if (mentionProfileCache.has(username)) {
			const cached = mentionProfileCache.get(username);
			if (cached) {
				mentionDictionary[username] = cached;
			}
			return cached;
		}

		if (mentionProfileInFlight.has(username)) {
			return mentionProfileInFlight.get(username);
		}

		const request = (async () => {
			try {
				const res = await fetch(`/api/users/${encodeURIComponent(username)}`);
				if (!res.ok) {
					mentionProfileCache.set(username, null);
					return null;
				}

				const profile = await res.json();
				profile.avatar_url = normalizeAvatarUrl(profile.avatar_url);

				mentionProfileCache.set(username, profile);
				mentionDictionary[username] = profile;
				return profile;
			} catch {
				mentionProfileCache.set(username, null);
				return null;
			} finally {
				mentionProfileInFlight.delete(username);
			}
		})();

		mentionProfileInFlight.set(username, request);
		return request;
	};

	const hydrateMentionDictionary = async (contents) => {
		const missing = new Set();
		contents.forEach((content) => {
			extractMentionUsernames(content).forEach((username) => {
				if (!mentionDictionary[username]) {
					missing.add(username);
				}
			});
		});

		if (missing.size === 0) return;

		await Promise.all([...missing].map((username) => ensureMentionProfile(username)));
	};

	const prepareComments = async (commentRows) => {
		await hydrateMentionDictionary(commentRows.map((comment) => comment.content));
		return commentRows.map((comment) => ({
			...comment,
			content: md.render(comment.content)
		}));
	};

	const getRootPageKey = (before) =>
		`${getPostId()}:${before == null || before === 0 ? 'start' : before}`;

	const getReplyPageKey = (parentId, before) =>
		`${getPostId()}:reply:${parentId}:${before == null || before === 0 ? 'start' : before}`;

	const getTextareaAnchorOffsetY = (node, anchorIndex) => {
		if (!node || anchorIndex == null || anchorIndex < 0) return null;

		const styles = window.getComputedStyle(node);
		const mirror = document.createElement('div');
		const span = document.createElement('span');
		const safeIndex = Math.min(anchorIndex, node.value.length);

		mirror.style.position = 'absolute';
		mirror.style.visibility = 'hidden';
		mirror.style.pointerEvents = 'none';
		mirror.style.whiteSpace = 'pre-wrap';
		mirror.style.overflowWrap = 'anywhere';
		mirror.style.wordBreak = 'break-word';
		mirror.style.boxSizing = styles.boxSizing;
		mirror.style.fontFamily = styles.fontFamily;
		mirror.style.fontSize = styles.fontSize;
		mirror.style.fontWeight = styles.fontWeight;
		mirror.style.fontStyle = styles.fontStyle;
		mirror.style.lineHeight = styles.lineHeight;
		mirror.style.letterSpacing = styles.letterSpacing;
		mirror.style.textTransform = styles.textTransform;
		mirror.style.textIndent = styles.textIndent;
		mirror.style.paddingTop = styles.paddingTop;
		mirror.style.paddingRight = styles.paddingRight;
		mirror.style.paddingBottom = styles.paddingBottom;
		mirror.style.paddingLeft = styles.paddingLeft;
		mirror.style.borderTopWidth = styles.borderTopWidth;
		mirror.style.borderRightWidth = styles.borderRightWidth;
		mirror.style.borderBottomWidth = styles.borderBottomWidth;
		mirror.style.borderLeftWidth = styles.borderLeftWidth;
		mirror.style.borderTopStyle = styles.borderTopStyle;
		mirror.style.borderRightStyle = styles.borderRightStyle;
		mirror.style.borderBottomStyle = styles.borderBottomStyle;
		mirror.style.borderLeftStyle = styles.borderLeftStyle;
		mirror.style.width = `${node.clientWidth}px`;

		mirror.textContent = node.value.slice(0, safeIndex);
		span.textContent = node.value[safeIndex] || '\u200b';
		mirror.append(span);

		document.body.append(mirror);

		const lineHeight = Number.parseFloat(styles.lineHeight);
		const fallbackLineHeight = Number.parseFloat(styles.fontSize) * 1.35;
		const effectiveLineHeight = Number.isFinite(lineHeight) ? lineHeight : fallbackLineHeight;
		const y = span.offsetTop - node.scrollTop + effectiveLineHeight + 6;

		mirror.remove();
		return y;
	};

	const schedulePopoverAnchor = () => {
		if (popoverAnchorRaf) {
			cancelAnimationFrame(popoverAnchorRaf);
		}

		popoverAnchorRaf = requestAnimationFrame(() => {
			const textarea = getTextarea();
			const composerSurface = getComposerSurface();
			if (!textarea || !composerSurface) {
				state.popoverTop = null;
				return;
			}

			const anchorIndex = commandState.open
				? commandState.start
				: mentionState.open
					? mentionState.start
					: -1;

			if (anchorIndex < 0) {
				state.popoverTop = null;
				return;
			}

			const textareaRect = textarea.getBoundingClientRect();
			const containerRect = composerSurface.getBoundingClientRect();
			const anchorOffset = getTextareaAnchorOffsetY(textarea, anchorIndex);

			if (anchorOffset == null) {
				state.popoverTop = null;
				return;
			}

			state.popoverTop = Math.max(0, textareaRect.top - containerRect.top + anchorOffset);
		});
	};

	const clearMentionState = () => {
		Object.assign(mentionState, createEmptyMentionState());
		state.popoverTop = null;
	};

	const clearCommandState = () => {
		Object.assign(commandState, createEmptyCommandState());
		state.popoverTop = null;
	};

	const getMentionContext = () => {
		const textarea = getTextarea();
		if (!textarea) return null;
		const caret = textarea.selectionStart ?? comments.current.length;
		const before = comments.current.slice(0, caret);
		const match = before.match(/(?:^|\s)@([A-Za-z0-9_-]+)$/);

		if (!match) return null;

		const query = match[1];
		if (query.length < mentionMinChars) return null;

		const fullMatch = match[0];
		const start = before.length - fullMatch.length + fullMatch.lastIndexOf('@');

		return { query, start, caret };
	};

	const resetKaomojiSearch = () => {
		state.kaomojiResults = [];
		state.kaomojiSuggestions = [];
		state.kaomojiPage = 1;
		state.kaomojiTotal = 0;
		state.kaomojiError = null;
	};

	const resetForPost = (postId) => {
		state.postId = postId;
		Object.assign(comments, createEmptyComments());
		resetObject(replyThreads, {});
		state.replyTo = null;
		clearCommandState();
		clearMentionState();
		state.showGifSearch = false;
		state.showKaomojiSearch = false;
		state.showMarkdownHelp = false;
		state.gifQuery = '';
		state.gifResults = [];
		state.gifLoading = false;
		state.gifOffset = 0;
		state.gifError = null;
		state.kaomojiMood = '';
		resetKaomojiSearch();

		rootPageCache.clear();
		rootPageInFlight.clear();
		replyPageCache.clear();
		replyPageInFlight.clear();
		mentionSearchCache.clear();
		mentionSearchInFlight.clear();
		commandKaomojiCache.clear();
		commandKaomojiInFlight.clear();
		commandGifCache.clear();
		commandGifInFlight.clear();
		drawerGifCache.clear();
		drawerGifInFlight.clear();
		drawerKaomojiCache.clear();
		drawerKaomojiInFlight.clear();
	};

	const fetchRootsPage = async (before) => {
		const key = getRootPageKey(before);

		if (rootPageCache.has(key)) {
			return rootPageCache.get(key);
		}

		if (rootPageInFlight.has(key)) {
			return rootPageInFlight.get(key);
		}

		const postId = getPostId();
		const api =
			before == null || before === 0
				? `/api/posts/id/${postId}/comments?limit=${rootLimit}`
				: `/api/posts/id/${postId}/comments?limit=${rootLimit}&before=${before}`;

		const request = (async () => {
			const res = await fetch(api);
			if (!res.ok) {
				return { comments: [], has_more: false };
			}

			const data = await res.json();
			const prepared = await prepareComments(data.comments ?? []);
			const page = { comments: prepared, has_more: Boolean(data.has_more) };
			rootPageCache.set(key, page);
			return page;
		})();

		rootPageInFlight.set(key, request);

		try {
			return await request;
		} finally {
			rootPageInFlight.delete(key);
		}
	};

	const fetchRepliesPage = async (parentId, before) => {
		const key = getReplyPageKey(parentId, before);

		if (replyPageCache.has(key)) {
			return replyPageCache.get(key);
		}

		if (replyPageInFlight.has(key)) {
			return replyPageInFlight.get(key);
		}

		const postId = getPostId();
		const api =
			before == null || before === 0
				? `/api/posts/id/${postId}/comments?parent_id=${parentId}&limit=${replyLimit}`
				: `/api/posts/id/${postId}/comments?parent_id=${parentId}&limit=${replyLimit}&before=${before}`;

		const request = (async () => {
			const res = await fetch(api);
			if (!res.ok) {
				return { comments: [], has_more: false };
			}

			const data = await res.json();
			const prepared = await prepareComments(data.comments ?? []);
			const page = { comments: prepared, has_more: Boolean(data.has_more) };
			replyPageCache.set(key, page);
			return page;
		})();

		replyPageInFlight.set(key, request);

		try {
			return await request;
		} finally {
			replyPageInFlight.delete(key);
		}
	};

	const searchMentionProfiles = async () => {
		const context = getMentionContext();

		if (!context) {
			clearMentionState();
			return;
		}

		if (
			mentionState.open &&
			mentionState.query === context.query &&
			mentionState.start === context.start
		) {
			return;
		}

		mentionState.query = context.query;
		mentionState.start = context.start;
		mentionState.loading = true;
		const requestId = mentionState.requestId + 1;
		mentionState.requestId = requestId;

		const cacheKey = context.query.trim().toLowerCase();
		if (mentionSearchCache.has(cacheKey)) {
			if (requestId !== mentionState.requestId) return;
			mentionState.items = mentionSearchCache.get(cacheKey);
			mentionState.selected = 0;
			mentionState.open = mentionState.items.length > 0;
			mentionState.loading = false;
			schedulePopoverAnchor();
			return;
		}

		if (mentionSearchInFlight.has(cacheKey)) {
			try {
				const items = await mentionSearchInFlight.get(cacheKey);
				if (requestId !== mentionState.requestId) return;
				mentionState.items = items;
				mentionState.selected = 0;
				mentionState.open = mentionState.items.length > 0;
				mentionState.loading = false;
				schedulePopoverAnchor();
				return;
			} catch {
				mentionSearchInFlight.delete(cacheKey);
				if (requestId === mentionState.requestId) {
					clearMentionState();
				}
				return;
			}
		}

		try {
			const request = (async () => {
				const res = await fetch(
					`/api/users?term=${encodeURIComponent(context.query)}&size=${mentionMaxProfiles}&offset=0`
				);

				if (!res.ok) {
					return [];
				}

				const data = await res.json();
				const items = (data.users ?? []).map((profile) => ({
					...profile,
					avatar_url: normalizeAvatarUrl(profile.avatar_url)
				}));

				mentionSearchCache.set(cacheKey, items);
				return items;
			})();

			mentionSearchInFlight.set(cacheKey, request);
			const items = await request;
			mentionSearchInFlight.delete(cacheKey);

			if (requestId !== mentionState.requestId) {
				return;
			}

			mentionState.items = items;
			mentionState.items.forEach((profile) => {
				mentionProfileCache.set(profile.username, profile);
				mentionDictionary[profile.username] = profile;
			});
			mentionState.selected = 0;
			mentionState.open = mentionState.items.length > 0;
			mentionState.loading = false;
			schedulePopoverAnchor();
		} catch {
			mentionSearchInFlight.delete(cacheKey);
			if (requestId === mentionState.requestId) {
				clearMentionState();
			}
		}
	};

	const searchKaomojis = async (context, requestId) => {
		const mood = context.query.trim().toLowerCase();
		if (!mood) {
			if (requestId === commandState.requestId) {
				commandState.items = [];
				commandState.suggestions = [];
				commandState.loading = false;
				commandState.error = null;
				commandState.open = true;
				schedulePopoverAnchor();
			}
			return;
		}

		const cacheKey = mood;
		if (commandKaomojiCache.has(cacheKey)) {
			if (requestId !== commandState.requestId) return;
			const cached = commandKaomojiCache.get(cacheKey);
			commandState.items = cached.items;
			commandState.suggestions = cached.suggestions;
			commandState.selected = 0;
			commandState.loading = false;
			commandState.error = cached.error;
			commandState.open =
				commandState.items.length > 0 ||
				commandState.suggestions.length > 0 ||
				Boolean(commandState.error);
			schedulePopoverAnchor();
			return;
		}

		try {
			let payload = null;
			let status = 200;

			if (commandKaomojiInFlight.has(cacheKey)) {
				({ payload, status } = await commandKaomojiInFlight.get(cacheKey));
			} else {
				const request = (async () => {
					const res = await fetch(
						`https://kaomoji-search.netlify.app/${encodeURIComponent(mood)}?page=1&limit=${commandMaxKaomojis}`
					);
					const body = await res.json().catch(() => ({}));
					return { payload: body, status: res.status };
				})();

				commandKaomojiInFlight.set(cacheKey, request);
				({ payload, status } = await request);
				commandKaomojiInFlight.delete(cacheKey);
			}

			if (requestId !== commandState.requestId) return;

			if (status === 404) {
				const suggestions = Array.isArray(payload?.suggestions)
					? payload.suggestions.filter(Boolean)
					: [];
				const cached = {
					items: [],
					suggestions,
					error: suggestions.length === 0 ? 'Mood not found.' : null
				};
				commandKaomojiCache.set(cacheKey, cached);

				commandState.items = cached.items;
				commandState.suggestions = cached.suggestions;
				commandState.selected = 0;
				commandState.loading = false;
				commandState.error = cached.error;
				commandState.open = suggestions.length > 0 || Boolean(commandState.error);
				schedulePopoverAnchor();
				return;
			}

			if (status < 200 || status >= 300) {
				commandState.items = [];
				commandState.suggestions = [];
				commandState.loading = false;
				commandState.error = 'Could not load kaomojis right now.';
				commandState.open = true;
				schedulePopoverAnchor();
				return;
			}

			const items = Array.isArray(payload?.results)
				? payload.results.filter(Boolean).map((value, index) => ({
						id: `kao-${index}-${value}`,
						value
					}))
				: [];
			const cached = {
				items,
				suggestions: [],
				error: items.length === 0 ? 'No kaomojis found for that mood.' : null
			};
			commandKaomojiCache.set(cacheKey, cached);

			commandState.items = cached.items;
			commandState.suggestions = cached.suggestions;
			commandState.selected = 0;
			commandState.loading = false;
			commandState.error = cached.error;
			commandState.open = items.length > 0 || Boolean(commandState.error);
			schedulePopoverAnchor();
		} catch {
			commandKaomojiInFlight.delete(cacheKey);
			if (requestId !== commandState.requestId) return;
			commandState.items = [];
			commandState.suggestions = [];
			commandState.loading = false;
			commandState.error = 'Could not load kaomojis right now.';
			commandState.open = true;
			schedulePopoverAnchor();
		}
	};

	const searchSyntaxGifs = async (context, requestId) => {
		const query = context.query.trim();
		if (!query) {
			if (requestId === commandState.requestId) {
				commandState.items = [];
				commandState.suggestions = [];
				commandState.loading = false;
				commandState.error = null;
				commandState.open = true;
				schedulePopoverAnchor();
			}
			return;
		}

		const cacheKey = query.toLowerCase();
		if (commandGifCache.has(cacheKey)) {
			if (requestId !== commandState.requestId) return;
			const cached = commandGifCache.get(cacheKey);
			commandState.items = cached.items;
			commandState.suggestions = [];
			commandState.selected = 0;
			commandState.loading = false;
			commandState.error = cached.error;
			commandState.open = commandState.items.length > 0 || Boolean(commandState.error);
			schedulePopoverAnchor();
			return;
		}

		try {
			let payload = null;
			let status = 200;

			if (commandGifInFlight.has(cacheKey)) {
				({ payload, status } = await commandGifInFlight.get(cacheKey));
			} else {
				const request = (async () => {
					const res = await fetch(`/api/gifs?q=${encodeURIComponent(query)}&offset=0`);
					const body = await res.json().catch(() => ({}));
					return { payload: body, status: res.status };
				})();

				commandGifInFlight.set(cacheKey, request);
				({ payload, status } = await request);
				commandGifInFlight.delete(cacheKey);
			}

			if (requestId !== commandState.requestId) return;

			if (status < 200 || status >= 300) {
				commandState.items = [];
				commandState.loading = false;
				commandState.error = 'Could not load GIF suggestions.';
				commandState.open = true;
				schedulePopoverAnchor();
				return;
			}

			const gifs = Array.isArray(payload?.data) ? payload.data.slice(0, commandMaxGifs) : [];
			const items = gifs.filter(
				(gif) => gif?.images?.original?.url && gif?.images?.fixed_height?.url
			);
			const cached = {
				items,
				error: items.length === 0 ? 'No GIFs found for that query.' : null
			};
			commandGifCache.set(cacheKey, cached);

			commandState.items = cached.items;
			commandState.suggestions = [];
			commandState.selected = 0;
			commandState.loading = false;
			commandState.error = cached.error;
			commandState.open = items.length > 0 || Boolean(commandState.error);
			schedulePopoverAnchor();
		} catch {
			commandGifInFlight.delete(cacheKey);
			if (requestId !== commandState.requestId) return;
			commandState.items = [];
			commandState.suggestions = [];
			commandState.loading = false;
			commandState.error = 'Could not load GIF suggestions.';
			commandState.open = true;
			schedulePopoverAnchor();
		}
	};

	const applyCommandReplacement = (context, replacement, nextCaretOffset = replacement.length) => {
		const textarea = getTextarea();
		if (!textarea || !context) return;

		comments.current = replaceCommandRange(comments.current, context, replacement);
		clearCommandState();
		syncTextareaLayout(context.start + nextCaretOffset);
	};

	const buildGifMarkdown = (gif) => {
		const gifUrl = gif?.images?.original?.url;
		if (!gifUrl) return null;

		const safeTitle = String(gif?.title ?? 'gif')
			.replace(/\[/g, ' ')
			.replace(/\]/g, ' ')
			.trim();

		return `![${safeTitle || 'gif'}](${gifUrl})`;
	};

	const searchCommandSuggestions = async () => {
		const textarea = getTextarea();
		if (!textarea) {
			clearCommandState();
			return;
		}

		const caret = textarea.selectionStart ?? comments.current.length;
		const context = getActiveCommentCommand(comments.current, caret);

		if (!context) {
			clearCommandState();
			return;
		}

		if (
			commandState.open &&
			commandState.query === context.query &&
			commandState.start === context.start &&
			commandState.type === context.kind
		) {
			return;
		}

		const wasOpen = commandState.open;
		clearMentionState();

		commandState.loading = true;
		commandState.open = true;
		commandState.type = context.kind;
		commandState.query = context.query;
		commandState.start = context.start;
		commandState.replaceEnd = context.replaceEnd;
		commandState.hasClosingParen = context.hasClosingParen;
		commandState.selected = 0;

		if (!wasOpen) {
			commandState.items = [];
			commandState.suggestions = [];
		}
		commandState.error = null;

		const requestId = commandState.requestId + 1;
		commandState.requestId = requestId;

		if (context.kind === COMMENT_COMMANDS.KAOMOJI) {
			await searchKaomojis(context, requestId);
			return;
		}

		if (context.kind === COMMENT_COMMANDS.GIF) {
			await searchSyntaxGifs(context, requestId);
			return;
		}

		clearCommandState();
	};

	const scheduleMentionSearch = () => {
		if (mentionDebounceTimer) {
			clearTimeout(mentionDebounceTimer);
		}

		mentionDebounceTimer = setTimeout(searchMentionProfiles, mentionDebounceMs);
	};

	const scheduleCommandSearch = () => {
		if (commandDebounceTimer) {
			clearTimeout(commandDebounceTimer);
		}

		commandDebounceTimer = setTimeout(searchCommandSuggestions, commandDebounceMs);
	};

	const syncTextareaLayout = (selectionStart = null, selectionEnd = selectionStart, focus = true) => {
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
		});
	};

	const insertTextAtSelection = (text) => {
		const textarea = getTextarea();
		if (!textarea) return;

		const startPos = textarea.selectionStart ?? comments.current.length;
		const endPos = textarea.selectionEnd ?? startPos;
		comments.current = textarea.value.slice(0, startPos) + text + textarea.value.slice(endPos);

		syncTextareaLayout(startPos + text.length);
	};

	const wrapSelection = (prefix, suffix = prefix, selectionOffset = prefix.length) => {
		const textarea = getTextarea();
		if (!textarea) return;

		const startPos = textarea.selectionStart ?? comments.current.length;
		const endPos = textarea.selectionEnd ?? startPos;
		comments.current =
			textarea.value.slice(0, startPos) +
			prefix +
			textarea.value.slice(startPos, endPos) +
			suffix +
			textarea.value.slice(endPos);

		syncTextareaLayout(startPos + selectionOffset, endPos + selectionOffset);
	};

	const insertAtCursor = (prefix, cursorOffset = prefix.length) => {
		const textarea = getTextarea();
		if (!textarea) return;

		const startPos = textarea.selectionStart ?? comments.current.length;
		const endPos = textarea.selectionEnd ?? startPos;
		comments.current = textarea.value.slice(0, startPos) + prefix + textarea.value.slice(endPos);

		syncTextareaLayout(startPos + cursorOffset, startPos + cursorOffset);
	};

	const clearReplyState = () => {
		state.replyTo = null;
	};

	const toggleMarkdownHelp = () => {
		state.showMarkdownHelp = !state.showMarkdownHelp;
		if (state.showMarkdownHelp) {
			state.showGifSearch = false;
			state.showKaomojiSearch = false;
			clearCommandState();
		}
	};

	const resetKaomojiSearchState = () => {
		state.kaomojiResults = [];
		state.kaomojiSuggestions = [];
		state.kaomojiPage = 1;
		state.kaomojiTotal = 0;
		state.kaomojiError = null;
	};

	const toggleKaomojiDrawer = () => {
		state.showKaomojiSearch = !state.showKaomojiSearch;
		state.showGifSearch = false;
		state.showMarkdownHelp = false;
		clearCommandState();
		if (!state.showKaomojiSearch) {
			resetKaomojiSearchState();
		}
	};

	const toggleGifDrawer = () => {
		state.showGifSearch = !state.showGifSearch;
		state.showKaomojiSearch = false;
		state.showMarkdownHelp = false;
		clearCommandState();
	};

	const fetchGifs = async (reset = false) => {
		if (state.gifLoading) return;
		state.gifLoading = true;
		state.gifError = null;

		const currentOffset = reset ? 0 : state.gifOffset;
		if (reset) {
			state.gifResults = [];
			state.gifOffset = 0;
		}

		const query = state.gifQuery.trim();
		const cacheKey = `${query.toLowerCase()}|${currentOffset}`;
		if (drawerGifCache.has(cacheKey)) {
			const cached = drawerGifCache.get(cacheKey);
			const nextGifs = cached.data;
			if (reset) {
				state.gifResults = nextGifs;
			} else {
				state.gifResults = [...state.gifResults, ...nextGifs];
			}
			state.gifOffset = currentOffset + nextGifs.length;
			state.gifLoading = false;
			return;
		}

		try {
			let payload = null;
			let status = 200;

			if (drawerGifInFlight.has(cacheKey)) {
				({ payload, status } = await drawerGifInFlight.get(cacheKey));
			} else {
				const request = (async () => {
					const url = `/api/gifs?q=${encodeURIComponent(state.gifQuery)}&offset=${currentOffset}`;
					const res = await fetch(url);
					const body = await res.json().catch(() => ({}));
					return { payload: body, status: res.status };
				})();

				drawerGifInFlight.set(cacheKey, request);
				({ payload, status } = await request);
				drawerGifInFlight.delete(cacheKey);
			}

			if (status < 200 || status >= 300) {
				throw new Error('Failed to load GIFs');
			}

			const newGifs = payload.data || [];
			drawerGifCache.set(cacheKey, { data: newGifs });

			if (reset) {
				state.gifResults = newGifs;
			} else {
				state.gifResults = [...state.gifResults, ...newGifs];
			}
			state.gifOffset = currentOffset + newGifs.length;
		} catch {
			drawerGifInFlight.delete(cacheKey);
			state.gifError = 'Could not load GIFs. Please check your connection.';
		} finally {
			state.gifLoading = false;
		}
	};

	const fetchKaomojis = async (reset = false) => {
		if (state.kaomojiLoading) return;

		const mood = state.kaomojiMood.trim().toLowerCase();
		if (!mood) {
			state.kaomojiError = null;
			return;
		}

		state.kaomojiLoading = true;
		state.kaomojiError = null;

		const nextPage = reset ? 1 : state.kaomojiPage;
		if (reset) {
			resetKaomojiSearchState();
		}

		const cacheKey = `${mood}|${nextPage}`;
		if (drawerKaomojiCache.has(cacheKey)) {
			const cached = drawerKaomojiCache.get(cacheKey);
			state.kaomojiSuggestions = cached.suggestions;
			state.kaomojiTotal = cached.total;
			state.kaomojiPage = nextPage + 1;

			if (reset) {
				state.kaomojiResults = cached.results;
			} else {
				state.kaomojiResults = [...state.kaomojiResults, ...cached.results];
			}

			state.kaomojiError = cached.error;
			state.kaomojiLoading = false;
			return;
		}

		try {
			let payload = null;
			let status = 200;

			if (drawerKaomojiInFlight.has(cacheKey)) {
				({ payload, status } = await drawerKaomojiInFlight.get(cacheKey));
			} else {
				const request = (async () => {
					const res = await fetch(
						`https://kaomoji-search.netlify.app/${encodeURIComponent(mood)}?page=${nextPage}&limit=18`
					);
					const body = await res.json().catch(() => ({}));
					return { payload: body, status: res.status };
				})();

				drawerKaomojiInFlight.set(cacheKey, request);
				({ payload, status } = await request);
				drawerKaomojiInFlight.delete(cacheKey);
			}

			if (status === 404) {
				const suggestions = Array.isArray(payload?.suggestions)
					? payload.suggestions.filter(Boolean)
					: [];
				drawerKaomojiCache.set(cacheKey, {
					results: [],
					suggestions,
					total: 0,
					error: suggestions.length === 0 ? 'Mood not found. Try another search.' : null
				});
				state.kaomojiResults = [];
				state.kaomojiSuggestions = suggestions;
				state.kaomojiError =
					state.kaomojiSuggestions.length === 0 ? 'Mood not found. Try another search.' : null;
				state.kaomojiTotal = 0;
				state.kaomojiPage = 1;
				return;
			}

			if (status < 200 || status >= 300) {
				throw new Error('Failed to load kaomojis');
			}

			const nextResults = Array.isArray(payload?.results) ? payload.results.filter(Boolean) : [];
			const total = Number(payload?.total ?? 0);
			drawerKaomojiCache.set(cacheKey, {
				results: nextResults,
				suggestions: [],
				total,
				error: nextResults.length === 0 ? 'No kaomojis found for that mood.' : null
			});

			state.kaomojiSuggestions = [];
			state.kaomojiTotal = total;
			state.kaomojiPage = nextPage + 1;

			if (reset) {
				state.kaomojiResults = nextResults;
			} else {
				state.kaomojiResults = [...state.kaomojiResults, ...nextResults];
			}

			if (state.kaomojiResults.length === 0) {
				state.kaomojiError = 'No kaomojis found for that mood.';
			}
		} catch {
			drawerKaomojiInFlight.delete(cacheKey);
			state.kaomojiError = 'Could not load kaomojis right now.';
		} finally {
			state.kaomojiLoading = false;
		}
	};

	const selectGif = (gif) => {
		const textarea = getTextarea();
		if (!textarea) return;
		const gifUrl = gif.images.original.url;
		const title = gif.title || 'gif';
		const markdownImage = `![${title}](${gifUrl})`;

		state.showGifSearch = false;
		insertTextAtSelection(markdownImage);
	};

	const selectKaomoji = (kaomoji) => {
		const textarea = getTextarea();
		if (!textarea || !kaomoji) return;
		state.showKaomojiSearch = false;
		insertTextAtSelection(`@@[${kaomoji}]@@ `);
	};

	const applyKaomojiMoodSuggestion = (suggestion) => {
		if (!suggestion) return;
		state.kaomojiMood = suggestion;
		fetchKaomojis(true);
	};

	const handleComposerInput = (event) => {
		if (event && event.type === 'keyup') {
			const ignoredKeys = [
				'ArrowDown',
				'ArrowUp',
				'Enter',
				'Escape',
				'Tab',
				'Shift',
				'Control',
				'Alt',
				'Meta',
				'CapsLock'
			];
			if (ignoredKeys.includes(event.key)) {
				schedulePopoverAnchor();
				return;
			}
		}

		const textarea = getTextarea();
		if (textarea) {
			const caret = textarea.selectionStart ?? comments.current.length;
			const context = getActiveCommentCommand(comments.current, caret);
			if (context) {
				clearMentionState();
				commandState.open = true;
				commandState.loading = true;
				commandState.type = context.kind;
				commandState.start = context.start;
				commandState.replaceEnd = context.replaceEnd;
			}
		}

		scheduleMentionSearch();
		scheduleCommandSearch();
		schedulePopoverAnchor();
	};

	const handleTextareaBlur = (event) => {
		setTimeout(() => {
			const activeEl = document.activeElement;
			if (activeEl && activeEl.closest('.comment-autocomplete-popover')) {
				return;
			}
			if (event.relatedTarget && event.relatedTarget.closest('.comment-autocomplete-popover')) {
				return;
			}
			clearMentionState();
			clearCommandState();
		}, 100);
	};

	const pickCommandItem = (item) => {
		if (commandState.start < 0) return;

		const context = {
			start: commandState.start,
			replaceEnd: commandState.replaceEnd,
			hasClosingParen: commandState.hasClosingParen
		};

		if (commandState.type === COMMENT_COMMANDS.KAOMOJI) {
			const kaomoji = typeof item === 'string' ? item : item?.value;
			if (!kaomoji) return;
			applyCommandReplacement(context, `@@[${kaomoji}]@@ `);
			return;
		}

		if (commandState.type === COMMENT_COMMANDS.GIF) {
			const markdownImage = buildGifMarkdown(item);
			if (!markdownImage) return;
			applyCommandReplacement(context, `${markdownImage} `);
		}
	};

	const applyKaomojiSuggestion = (suggestion) => {
		const textarea = getTextarea();
		if (!textarea || commandState.start < 0 || commandState.type !== COMMENT_COMMANDS.KAOMOJI)
			return;

		const context = {
			start: commandState.start,
			replaceEnd: commandState.replaceEnd,
			hasClosingParen: commandState.hasClosingParen
		};

		const token = buildCommandToken(COMMENT_COMMANDS.KAOMOJI, suggestion, context.hasClosingParen);
		comments.current = replaceCommandRange(comments.current, context, token);

		requestAnimationFrame(() => {
			const nextCaret = context.start + token.length - (context.hasClosingParen ? 1 : 0);
			resizeTextarea(textarea);
			textarea.focus();
			textarea.setSelectionRange(nextCaret, nextCaret);
			searchCommandSuggestions();
		});
	};

	const pickMention = (pickedUser) => {
		const textarea = getTextarea();
		if (!textarea || mentionState.start < 0) return;

		const caret = textarea.selectionStart ?? comments.current.length;
		const left = comments.current.slice(0, mentionState.start + 1);
		const right = comments.current.slice(caret);
		const injected = `${left}${pickedUser.username} ${right}`;

		comments.current = injected;
		clearMentionState();
		syncTextareaLayout(left.length + pickedUser.username.length + 1);
	};

	const handleTextareaKeydown = (event) => {
		const textarea = getTextarea();
		if (!textarea) return;

		if (event.key === 'Home') {
			event.preventDefault();
			event.stopPropagation();
			const text = textarea.value;
			const cursor = textarea.selectionStart;
			const lineStart = text.lastIndexOf('\n', cursor - 1) + 1;
			if (event.shiftKey) {
				textarea.setSelectionRange(lineStart, textarea.selectionEnd);
			} else {
				textarea.setSelectionRange(lineStart, lineStart);
			}
			return;
		}

		if (event.key === 'End') {
			event.preventDefault();
			event.stopPropagation();
			const text = textarea.value;
			const cursor = textarea.selectionEnd;
			let lineEnd = text.indexOf('\n', cursor);
			if (lineEnd === -1) lineEnd = text.length;
			if (event.shiftKey) {
				textarea.setSelectionRange(textarea.selectionStart, lineEnd);
			} else {
				textarea.setSelectionRange(lineEnd, lineEnd);
			}
			return;
		}

		if (event.key === 'PageUp') {
			event.preventDefault();
			event.stopPropagation();
			const text = textarea.value;
			const cursor = textarea.selectionStart;
			let pos = cursor;
			for (let i = 0; i < 10; i++) {
				const prev = text.lastIndexOf('\n', pos - 1);
				if (prev === -1) {
					pos = 0;
					break;
				}
				pos = prev;
			}
			if (event.shiftKey) {
				textarea.setSelectionRange(pos, textarea.selectionEnd);
			} else {
				textarea.setSelectionRange(pos, pos);
			}
			return;
		}

		if (event.key === 'PageDown') {
			event.preventDefault();
			event.stopPropagation();
			const text = textarea.value;
			const cursor = textarea.selectionEnd;
			let pos = cursor;
			for (let i = 0; i < 10; i++) {
				const next = text.indexOf('\n', pos + 1);
				if (next === -1) {
					pos = text.length;
					break;
				}
				pos = next;
			}
			if (event.shiftKey) {
				textarea.setSelectionRange(textarea.selectionStart, pos);
			} else {
				textarea.setSelectionRange(pos, pos);
			}
			return;
		}

		if (event.ctrlKey || event.metaKey) {
			const key = event.key.toLowerCase();
			if (key === 'b') {
				event.preventDefault();
				wrapSelection('**');
				return;
			}
			if (key === 'i') {
				event.preventDefault();
				wrapSelection('_');
				return;
			}
			if (key === 'e') {
				event.preventDefault();
				wrapSelection('`');
				return;
			}
		}

		if (commandState.open) {
			const activeItems =
				commandState.items.length > 0 ? commandState.items : commandState.suggestions;

			if (event.key === 'ArrowDown') {
				event.preventDefault();
				if (activeItems.length > 0) {
					commandState.selected = (commandState.selected + 1) % activeItems.length;
				}
				return;
			}

			if (event.key === 'ArrowUp') {
				event.preventDefault();
				if (activeItems.length > 0) {
					commandState.selected =
						(commandState.selected - 1 + activeItems.length) % activeItems.length;
				}
				return;
			}

			if (event.key === 'Enter' || event.key === 'Tab') {
				if (commandState.items.length > 0) {
					event.preventDefault();
					pickCommandItem(commandState.items[commandState.selected]);
					return;
				}

				if (commandState.suggestions.length > 0) {
					event.preventDefault();
					applyKaomojiSuggestion(
						commandState.suggestions[commandState.selected] ?? commandState.suggestions[0]
					);
					return;
				}
			}

			if (event.key === 'Escape') {
				event.preventDefault();
				clearCommandState();
				return;
			}
		}

		if (!mentionState.open || mentionState.items.length === 0) return;

		if (event.key === 'ArrowDown') {
			event.preventDefault();
			if (mentionState.items.length > 0) {
				mentionState.selected = (mentionState.selected + 1) % mentionState.items.length;
			}
			return;
		}

		if (event.key === 'ArrowUp') {
			event.preventDefault();
			if (mentionState.items.length > 0) {
				mentionState.selected =
					(mentionState.selected - 1 + mentionState.items.length) % mentionState.items.length;
			}
			return;
		}

		if (event.key === 'Enter' || event.key === 'Tab') {
			event.preventDefault();
			pickMention(mentionState.items[mentionState.selected]);
			return;
		}

		if (event.key === 'Escape') {
			event.preventDefault();
			clearMentionState();
		}
	};

	const updateRoots = (newComments, hasMore) => {
		const merged = new Map();
		[...comments.roots, ...newComments].forEach((comment) => {
			merged.set(comment.id, comment);
		});

		comments.roots = [...merged.values()].sort((a, b) => b.id - a.id);

		const length = comments.roots.length;

		if (length > 0) {
			comments.lastId = comments.roots[length - 1].id;
		}

		comments.endReached = !hasMore;
	};

	const ensureReplyThread = (parentId, total = 0) => {
		if (!replyThreads[parentId]) {
			replyThreads[parentId] = {
				expanded: false,
				fetching: false,
				endReached: true,
				lastId: 0,
				total,
				items: []
			};
		} else if (total != null && total >= 0) {
			replyThreads[parentId].total = total;
		}

		return replyThreads[parentId];
	};

	const updateReplyItems = (parentId, newReplies, hasMore) => {
		const thread = ensureReplyThread(parentId);
		const merged = new Map();
		[...thread.items, ...newReplies].forEach((reply) => {
			merged.set(reply.id, reply);
		});

		thread.items = [...merged.values()].sort((a, b) => b.id - a.id);
		if (thread.items.length > 0) {
			thread.lastId = thread.items[thread.items.length - 1].id;
		}

		thread.endReached = !hasMore;
	};

	const loadMoreReplies = async (parentId) => {
		const thread = ensureReplyThread(parentId);
		if (thread.fetching || thread.endReached) return;

		thread.fetching = true;
		const before = thread.lastId === 0 ? null : thread.lastId;
		const page = await fetchRepliesPage(parentId, before);
		updateReplyItems(parentId, page.comments, page.has_more);
		page.comments.forEach((reply) => {
			ensureReplyThread(reply.id, reply.direct_reply_count ?? 0);
		});
		thread.fetching = false;
	};

	const toggleReplies = async (comment) => {
		const total = comment.direct_reply_count ?? 0;
		const thread = ensureReplyThread(comment.id, total);
		thread.expanded = !thread.expanded;

		if (thread.expanded && thread.items.length === 0 && total > 0) {
			thread.endReached = false;
			await loadMoreReplies(comment.id);
		}
	};

	const findCommentById = (id) => {
		if (id == null) return null;

		const root = comments.roots.find((item) => item.id === id);
		if (root) return root;

		for (const parentId of Object.keys(replyThreads)) {
			const thread = replyThreads[parentId];
			const found = thread.items.find((item) => item.id === id);
			if (found) return found;
		}

		return null;
	};

	const expandReplyChain = async (commentId) => {
		let currentId = commentId;

		while (currentId != null) {
			const current = findCommentById(currentId);
			if (!current) break;

			const thread = ensureReplyThread(currentId, current.direct_reply_count ?? 0);
			thread.expanded = true;

			if (thread.items.length === 0 && (current.direct_reply_count ?? 0) > 0) {
				thread.endReached = false;
				await loadMoreReplies(currentId);
			}

			currentId = current.parent_id ?? null;
		}
	};

	const handleReply = (comment, rootId) => {
		state.replyTo = {
			...comment,
			rootId
		};
		getTextarea()?.focus();
	};

	const fetchComments = async () => {
		if (comments.fetching) return;
		comments.fetching = true;
		const nextPage = await fetchRootsPage(comments.lastId === 0 ? null : comments.lastId);
		updateRoots(nextPage.comments, nextPage.has_more);
		nextPage.comments.forEach((root) => {
			ensureReplyThread(root.id, root.direct_reply_count ?? 0);
		});
		comments.fetching = false;
	};

	const submitComment = async () => {
		if (comments.sending || comments.current.length < 1) return;

		const content = comments.current;
		const currentUser = get(user);
		const headers =
			currentUser !== undefined
				? {
						Authorization: auth(),
						'Content-Type': 'application/json'
					}
				: {
						'Content-Type': 'application/json'
					};

		comments.sending = true;

		try {
			const postId = getPostId();
			const res = await fetch(`/api/posts/id/${postId}/comments/new`, {
				method: 'PUT',
				headers,
				body: JSON.stringify({
					content,
					parent_id: state.replyTo?.id ?? null
				})
			});

			if (!res.ok) return;

			const data = await res.json();
			const userData =
				currentUser !== undefined
					? {
							display_name: currentUser.displayName,
							username: currentUser.username,
							user_role: currentUser.role
						}
					: {};
			const newComment = {
				id: data.comment_id,
				avatar_url: getUserAvatarUrl(),
				content,
				parent_id: state.replyTo?.id ?? null,
				direct_reply_count: 0,
				created_at: undefined,
				...userData
			};

			comments.current = '';
			const textarea = getTextarea();
			if (textarea) {
				resizeTextarea(textarea);
			}
			await hydrateMentionDictionary([newComment.content]);
			newComment.content = md.render(newComment.content);

			if (newComment.parent_id == null) {
				updateRoots([newComment], !comments.endReached);
				ensureReplyThread(newComment.id, 0);
			} else {
				const parentId = newComment.parent_id;
				await expandReplyChain(parentId);

				const parentComment = findCommentById(parentId);
				if (parentComment) {
					const previous = parentComment.direct_reply_count ?? 0;
					parentComment.direct_reply_count = previous + 1;
				}

				const thread = ensureReplyThread(parentId, parentComment?.direct_reply_count ?? 1);
				thread.total = Math.max(thread.total ?? 0, 1);
				updateReplyItems(parentId, [newComment], true);
				thread.endReached = thread.items.length >= (thread.total ?? 0);
			}

			state.replyTo = null;
			clearMentionState();
			clearCommandState();
		} finally {
			comments.sending = false;
		}
	};

	const handlePostChange = (postId) => {
		resetForPost(postId);

		const start = getStart();
		if (!start) return () => {};

		const onScrolled = gsap.to(start, {
			scrollTrigger: {
				trigger: start,
				once: true,
				start: 'bottom bottom',
				onEnter: fetchComments
			}
		});

		const triggerInstance = onScrolled.scrollTrigger;

		return () => {
			triggerInstance?.kill();
			onScrolled?.kill();
		};
	};

	const closeMarkdownHelp = () => {
		state.showMarkdownHelp = false;
	};

	return {
		state,
		md,
		handlePostChange,
		handleComposerInput,
		handleTextareaBlur,
		handleTextareaKeydown,
		fetchComments,
		handleReply,
		toggleReplies,
		loadMoreReplies,
		submitComment,
		toggleMarkdownHelp,
		toggleKaomojiDrawer,
		toggleGifDrawer,
		closeMarkdownHelp,
		insertAtCursor,
		wrapSelection,
		pickCommandItem,
		applyKaomojiSuggestion,
		pickMention,
		schedulePopoverAnchor,
		fetchGifs,
		fetchKaomojis,
		selectGif,
		selectKaomoji,
		applyKaomojiMoodSuggestion,
	};
};

export { createEmptyState };
