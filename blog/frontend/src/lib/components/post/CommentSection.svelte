<script>
	import { autoHResize, resizeTextarea } from '$lib/client/auto-resize';
	import { auth, user } from '$lib/client/user';
	import { gsap } from 'gsap';
	import MarkdownIt from 'markdown-it';
	import Comment from './Comment.svelte';
	import CommentAutocompletePopover from './CommentAutocompletePopover.svelte';
	import CommentComposerToolbar from './CommentComposerToolbar.svelte';
	import CommentGifDrawer from './CommentGifDrawer.svelte';
	import CommentKaomojiDrawer from './CommentKaomojiDrawer.svelte';
	import CommentMarkdownHelp from './CommentMarkdownHelp.svelte';
	import CommentThread from './CommentThread.svelte';
	import {
		buildCommandToken,
		COMMENT_COMMANDS,
		getActiveCommentCommand,
		replaceCommandRange
	} from './comment-syntax';
	import { codeHighlightPlugin, mentionProfilePlugin, kaomojiPlugin } from '$lib/custom-rules';

	const rootLimit = 3;
	const replyLimit = 5;
	const mentionDictionary = {};
	const mentionProfileCache = new Map();
	const mentionProfileInFlight = new Map();
	const mentionSearchCache = new Map();
	const mentionSearchInFlight = new Map();
	const rootPageCache = new Map();
	const rootPageInFlight = new Map();
	const replyPageCache = new Map();
	const replyPageInFlight = new Map();
	const commandKaomojiCache = new Map();
	const commandKaomojiInFlight = new Map();
	const commandGifCache = new Map();
	const commandGifInFlight = new Map();
	const drawerGifCache = new Map();
	const drawerGifInFlight = new Map();
	const drawerKaomojiCache = new Map();
	const drawerKaomojiInFlight = new Map();
	const md = new MarkdownIt()
		.use(codeHighlightPlugin)
		.use(mentionProfilePlugin, { mentionDictionary })
		.use(kaomojiPlugin);
	const mentionDebounceMs = 250;
	const mentionMinChars = 3;
	const mentionMaxProfiles = 5;
	const mentionRegex = /(^|[\s(>])@([A-Za-z0-9_-]{3,32})\b/g;
	const commandDebounceMs = 250;
	const commandMaxKaomojis = 12;
	const commandMaxGifs = 12;

	let { postId, postAuthorUsername = null } = $props();
	let last = -1;

	let userAvatarUrl = $derived($user?.avatarUrl ?? '/anonymous.gif');

	let start = $state();

	let comments = $state({
		current: '',
		fetching: false,
		sending: false,
		endReached: false,
		lastId: 0,
		roots: []
	});
	let replyThreads = $state({});

	let textarea = $state();
	let replyTo = $state(null);
	let mentionState = $state({
		open: false,
		loading: false,
		query: '',
		start: -1,
		selected: 0,
		items: [],
		requestId: 0
	});
	let mentionDebounceTimer = $state();
	let commandState = $state({
		open: false,
		loading: false,
		type: '',
		query: '',
		start: -1,
		replaceEnd: -1,
		hasClosingParen: false,
		selected: 0,
		items: [],
		suggestions: [],
		error: null,
		requestId: 0
	});
	let commandDebounceTimer = $state();
	let composerSurface;
	let popoverTop = $state(null);
	let popoverAnchorRaf;

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
			if (!textarea || !composerSurface) {
				popoverTop = null;
				return;
			}

			const anchorIndex = commandState.open
				? commandState.start
				: mentionState.open
					? mentionState.start
					: -1;

			if (anchorIndex < 0) {
				popoverTop = null;
				return;
			}

			const textareaRect = textarea.getBoundingClientRect();
			const containerRect = composerSurface.getBoundingClientRect();
			const anchorOffset = getTextareaAnchorOffsetY(textarea, anchorIndex);

			if (anchorOffset == null) {
				popoverTop = null;
				return;
			}

			popoverTop = Math.max(0, textareaRect.top - containerRect.top + anchorOffset);
		});
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

	const normalizeAvatarUrl = (url) => {
		if (!url) return '/anonymous.gif';
		if (url.startsWith('http://') || url.startsWith('https://')) return url;
		if (url.startsWith('/api/') || url.startsWith('/')) return url;
		return `/api/${url.replace(/^\.?\//, '')}`;
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

	const prepareComments = async (commentRows) => {
		await hydrateMentionDictionary(commentRows.map((comment) => comment.content));
		return commentRows.map((comment) => ({
			...comment,
			content: md.render(comment.content)
		}));
	};

	const getRootPageKey = (before) =>
		`${postId}:${before == null || before === 0 ? 'start' : before}`;

	const getReplyPageKey = (parentId, before) =>
		`${postId}:reply:${parentId}:${before == null || before === 0 ? 'start' : before}`;

	const fetchRootsPage = async (before) => {
		const key = getRootPageKey(before);

		if (rootPageCache.has(key)) {
			return rootPageCache.get(key);
		}

		if (rootPageInFlight.has(key)) {
			return rootPageInFlight.get(key);
		}

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

	const resetMentionState = () => {
		mentionState.open = false;
		mentionState.loading = false;
		mentionState.query = '';
		mentionState.start = -1;
		mentionState.selected = 0;
		mentionState.items = [];
		popoverTop = null;
	};

	const resetCommandState = () => {
		commandState.open = false;
		commandState.loading = false;
		commandState.type = '';
		commandState.query = '';
		commandState.start = -1;
		commandState.replaceEnd = -1;
		commandState.hasClosingParen = false;
		commandState.selected = 0;
		commandState.items = [];
		commandState.suggestions = [];
		commandState.error = null;
		popoverTop = null;
	};

	const getMentionContext = () => {
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

	const searchMentionProfiles = async () => {
		const context = getMentionContext();

		if (!context) {
			resetMentionState();
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
					resetMentionState();
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
				resetMentionState();
			}
		}
	};

	const scheduleMentionSearch = () => {
		if (mentionDebounceTimer) {
			clearTimeout(mentionDebounceTimer);
		}

		mentionDebounceTimer = setTimeout(searchMentionProfiles, mentionDebounceMs);
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

	const applyCommandReplacement = (context, replacement, nextCaretOffset = replacement.length) => {
		if (!textarea || !context) return;

		comments.current = replaceCommandRange(comments.current, context, replacement);
		resetCommandState();
		syncTextareaLayout(context.start + nextCaretOffset);
	};

	const searchKaomojis = async (context, requestId) => {
		const mood = context.query.trim().toLowerCase();
		if (!mood) {
			if (requestId === commandState.requestId) resetCommandState();
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
			if (requestId === commandState.requestId) resetCommandState();
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

	const searchCommandSuggestions = async () => {
		if (!textarea) {
			resetCommandState();
			return;
		}

		const caret = textarea.selectionStart ?? comments.current.length;
		const context = getActiveCommentCommand(comments.current, caret);

		if (!context) {
			resetCommandState();
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

		resetMentionState();

		commandState.loading = true;
		commandState.open = false;
		commandState.type = context.kind;
		commandState.query = context.query;
		commandState.start = context.start;
		commandState.replaceEnd = context.replaceEnd;
		commandState.hasClosingParen = context.hasClosingParen;
		commandState.selected = 0;
		commandState.items = [];
		commandState.suggestions = [];
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

		resetCommandState();
	};

	const scheduleCommandSearch = () => {
		if (commandDebounceTimer) {
			clearTimeout(commandDebounceTimer);
		}

		commandDebounceTimer = setTimeout(searchCommandSuggestions, commandDebounceMs);
	};

	const applyKaomojiSuggestion = (suggestion) => {
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
		scheduleMentionSearch();
		scheduleCommandSearch();
		schedulePopoverAnchor();
	};

	const pickMention = (pickedUser) => {
		if (!textarea || mentionState.start < 0) return;

		const caret = textarea.selectionStart ?? comments.current.length;
		const left = comments.current.slice(0, mentionState.start + 1);
		const right = comments.current.slice(caret);
		const injected = `${left}${pickedUser.username} ${right}`;

		comments.current = injected;
		resetMentionState();
		syncTextareaLayout(left.length + pickedUser.username.length + 1);
	};

	const handleTextareaKeydown = (event) => {
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
				resetCommandState();
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
			resetMentionState();
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
		replyTo = {
			...comment,
			rootId
		};
		textarea?.focus();
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

	$effect(() => {
		if (last !== postId) {
			last = postId;

			comments = {
				current: '',
				fetching: false,
				sending: false,
				endReached: false,
				lastId: 0,
				roots: []
			};
			replyThreads = {};
			replyTo = null;
			resetCommandState();
			resetMentionState();

			rootPageCache.clear();
			rootPageInFlight.clear();
			replyPageCache.clear();
			replyPageInFlight.clear();

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
		}
	});

	let showGifSearch = $state(false);
	let showKaomojiSearch = $state(false);
	let gifQuery = $state('');
	let gifResults = $state([]);
	let gifLoading = $state(false);
	let gifOffset = $state(0);
	let gifError = $state(null);
	let kaomojiMood = $state('');
	let kaomojiResults = $state([]);
	let kaomojiSuggestions = $state([]);
	let kaomojiLoading = $state(false);
	let kaomojiPage = $state(1);
	let kaomojiTotal = $state(0);
	let kaomojiError = $state(null);
	let showMarkdownHelp = $state(false);

	const syncTextareaLayout = (
		selectionStart = null,
		selectionEnd = selectionStart,
		focus = true
	) => {
		requestAnimationFrame(() => {
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
		if (!textarea) return;

		const startPos = textarea.selectionStart ?? comments.current.length;
		const endPos = textarea.selectionEnd ?? startPos;
		comments.current = textarea.value.slice(0, startPos) + text + textarea.value.slice(endPos);

		syncTextareaLayout(startPos + text.length);
	};

	const wrapSelection = (prefix, suffix = prefix, selectionOffset = prefix.length) => {
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
		if (!textarea) return;

		const startPos = textarea.selectionStart ?? comments.current.length;
		const endPos = textarea.selectionEnd ?? startPos;
		comments.current = textarea.value.slice(0, startPos) + prefix + textarea.value.slice(endPos);

		syncTextareaLayout(startPos + cursorOffset, startPos + cursorOffset);
	};

	const toggleMarkdownHelp = () => {
		showMarkdownHelp = !showMarkdownHelp;
		if (showMarkdownHelp) {
			showGifSearch = false;
			showKaomojiSearch = false;
			resetCommandState();
		}
	};

	const toggleKaomojiDrawer = () => {
		showKaomojiSearch = !showKaomojiSearch;
		showGifSearch = false;
		showMarkdownHelp = false;
		resetCommandState();
		if (!showKaomojiSearch) {
			resetKaomojiSearch();
		}
	};

	const toggleGifDrawer = () => {
		showGifSearch = !showGifSearch;
		showKaomojiSearch = false;
		showMarkdownHelp = false;
		resetCommandState();
	};

	const resetKaomojiSearch = () => {
		kaomojiResults = [];
		kaomojiSuggestions = [];
		kaomojiPage = 1;
		kaomojiTotal = 0;
		kaomojiError = null;
	};

	const fetchGifs = async (reset = false) => {
		if (gifLoading) return;
		gifLoading = true;
		gifError = null;

		const currentOffset = reset ? 0 : gifOffset;
		if (reset) {
			gifResults = [];
			gifOffset = 0;
		}

		const query = gifQuery.trim();
		const cacheKey = `${query.toLowerCase()}|${currentOffset}`;
		if (drawerGifCache.has(cacheKey)) {
			const cached = drawerGifCache.get(cacheKey);
			const nextGifs = cached.data;
			if (reset) {
				gifResults = nextGifs;
			} else {
				gifResults = [...gifResults, ...nextGifs];
			}
			gifOffset = currentOffset + nextGifs.length;
			gifLoading = false;
			return;
		}

		try {
			let payload = null;
			let status = 200;

			if (drawerGifInFlight.has(cacheKey)) {
				({ payload, status } = await drawerGifInFlight.get(cacheKey));
			} else {
				const request = (async () => {
					const url = `/api/gifs?q=${encodeURIComponent(gifQuery)}&offset=${currentOffset}`;
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
				gifResults = newGifs;
			} else {
				gifResults = [...gifResults, ...newGifs];
			}
			gifOffset = currentOffset + newGifs.length;
		} catch (err) {
			drawerGifInFlight.delete(cacheKey);
			gifError = 'Could not load GIFs. Please check your connection.';
		} finally {
			gifLoading = false;
		}
	};

	const fetchKaomojis = async (reset = false) => {
		if (kaomojiLoading) return;

		const mood = kaomojiMood.trim().toLowerCase();
		if (!mood) {
			kaomojiError = null;
			return;
		}

		kaomojiLoading = true;
		kaomojiError = null;

		const nextPage = reset ? 1 : kaomojiPage;
		if (reset) {
			resetKaomojiSearch();
		}

		const cacheKey = `${mood}|${nextPage}`;
		if (drawerKaomojiCache.has(cacheKey)) {
			const cached = drawerKaomojiCache.get(cacheKey);
			kaomojiSuggestions = cached.suggestions;
			kaomojiTotal = cached.total;
			kaomojiPage = nextPage + 1;

			if (reset) {
				kaomojiResults = cached.results;
			} else {
				kaomojiResults = [...kaomojiResults, ...cached.results];
			}

			kaomojiError = cached.error;
			kaomojiLoading = false;
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
				kaomojiResults = [];
				kaomojiSuggestions = suggestions;
				kaomojiError =
					kaomojiSuggestions.length === 0 ? 'Mood not found. Try another search.' : null;
				kaomojiTotal = 0;
				kaomojiPage = 1;
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

			kaomojiSuggestions = [];
			kaomojiTotal = total;
			kaomojiPage = nextPage + 1;

			if (reset) {
				kaomojiResults = nextResults;
			} else {
				kaomojiResults = [...kaomojiResults, ...nextResults];
			}

			if (kaomojiResults.length === 0) {
				kaomojiError = 'No kaomojis found for that mood.';
			}
		} catch {
			drawerKaomojiInFlight.delete(cacheKey);
			kaomojiError = 'Could not load kaomojis right now.';
		} finally {
			kaomojiLoading = false;
		}
	};

	const selectGif = (gif) => {
		if (!textarea) return;
		const gifUrl = gif.images.original.url;
		const title = gif.title || 'gif';
		const markdownImage = `![${title}](${gifUrl})`;

		showGifSearch = false; // Close drawer after selection
		insertTextAtSelection(markdownImage);
	};

	const selectKaomoji = (kaomoji) => {
		if (!textarea || !kaomoji) return;
		showKaomojiSearch = false;
		insertTextAtSelection(`@@[${kaomoji}]@@ `);
	};

	const applyKaomojiMoodSuggestion = (suggestion) => {
		if (!suggestion) return;
		kaomojiMood = suggestion;
		fetchKaomojis(true);
	};

	$effect(() => {
		if (showGifSearch && gifResults.length === 0) {
			fetchGifs(true);
		}
	});

	$effect(() => {
		if (!showKaomojiSearch) return;
		if (!kaomojiMood.trim()) return;
		if (!kaomojiLoading && kaomojiResults.length === 0 && kaomojiSuggestions.length === 0) {
			fetchKaomojis(true);
		}
	});

	$effect(() => {
		if (commandState.open || mentionState.open) {
			schedulePopoverAnchor();
		}
	});

	const closeMarkdownHelp = () => {
		showMarkdownHelp = false;
	};
</script>

<section class="w-full xl:max-w-[calc(100%-15rem)] xl:w-[calc(100%-15rem)] bg-white p-4 rounded-xl">
	<h4 class="text-lg lg:text-2xl">Join the discussion!</h4>
	<div class="flex flex-col gap-4" bind:this={start}>
		<hr class="border-t-3 border-dark mb-6" />
		<div class="flex gap-8">
			<div
				class="not-xxs:hidden min-w-12 max-w-12 lg:min-w-20 lg:max-w-20 h-12 lg:h-20 outline-primary outline-3 rounded-full overflow-hidden"
			>
				<img class="full object-cover" src={userAvatarUrl} alt="comment-posting-avatar" />
			</div>
			<div class="grow min-w-0 flex flex-col gap-4 relative">
				<svg
					class="not-xxs:hidden absolute fill-primary top-6 lg:top-10 -left-4 -translate-y-1/2 w-4 h-4"
					viewBox="0 0 24 24"
				>
					<polygon points="0,12 24,0 24,24" />
				</svg>
				<div class="w-full max-w-full">
					<div
						class="relative text-base w-full bg-primary-20 border-primary border-2 border-b-0 rounded-t-xl"
						bind:this={composerSurface}
					>
						<textarea
							name="comment-input"
							class="comment-input block w-full min-h-16 lg:min-h-20 max-w-full overflow-hidden outline-none resize-none p-2 bg-transparent"
							wrap="soft"
							bind:this={textarea}
							bind:value={comments.current}
							oninput={handleComposerInput}
							onclick={handleComposerInput}
							onkeyup={handleComposerInput}
							onscroll={schedulePopoverAnchor}
							onkeydown={handleTextareaKeydown}
							onblur={(event) => {
								setTimeout(() => {
									const activeEl = document.activeElement;
									if (activeEl && activeEl.closest('.comment-autocomplete-popover')) {
										return;
									}
									if (
										event.relatedTarget &&
										event.relatedTarget.closest('.comment-autocomplete-popover')
									) {
										return;
									}
									resetMentionState();
									resetCommandState();
								}, 100);
							}}
							{@attach autoHResize}
						></textarea>

						<CommentGifDrawer
							show={showGifSearch}
							{gifQuery}
							onGifQueryInput={(value) => {
								gifQuery = value;
							}}
							{gifResults}
							{gifLoading}
							{gifError}
							{fetchGifs}
							{selectGif}
						/>

						<CommentKaomojiDrawer
							show={showKaomojiSearch}
							{kaomojiMood}
							onKaomojiMoodInput={(value) => {
								kaomojiMood = value;
							}}
							{kaomojiResults}
							{kaomojiSuggestions}
							{kaomojiLoading}
							{kaomojiTotal}
							{kaomojiError}
							{fetchKaomojis}
							{selectKaomoji}
							{applyKaomojiMoodSuggestion}
						/>

						<div class="border-t-2 border-primary/15 bg-white/40 p-3 flex flex-col gap-2">
							<span class="text-base font-semibold text-primary/70 select-none">Live Preview</span>
							<div class="w-full min-h-12 overflow-hidden">
								{#if comments.current.trim().length > 0}
									<Comment content={md.render(comments.current)} />
								{:else}
									<p class="text-sm text-dark/40 italic select-none">
										Nothing to preview yet. Start typing... <span aria-hidden="true">ヽ(ヅ)ノ</span>
									</p>
								{/if}
							</div>
						</div>

						<CommentAutocompletePopover
							{commandState}
							{mentionState}
							{popoverTop}
							{pickCommandItem}
							{applyKaomojiSuggestion}
							{pickMention}
						/>
					</div>
					<CommentComposerToolbar
						{showMarkdownHelp}
						{showKaomojiSearch}
						{showGifSearch}
						onToggleHelp={toggleMarkdownHelp}
						onHeader={() => insertAtCursor('# ', 2)}
						onBold={() => wrapSelection('**')}
						onItalic={() => wrapSelection('_')}
						onCode={() => wrapSelection('`')}
						onToggleKaomoji={toggleKaomojiDrawer}
						onToggleGif={toggleGifDrawer}
					/>
				</div>
				<CommentMarkdownHelp open={showMarkdownHelp} close={closeMarkdownHelp} />
				{#if replyTo}
					<div
						class="flex items-center justify-between gap-4 rounded-xl border-2 border-dark/20 bg-primary-20 px-3 py-2"
					>
						<span class="text-sm text-dark/80">
							Replying to {replyTo.display_name ?? replyTo.username ?? 'comment'}
						</span>
						<button
							type="button"
							class="text-sm text-dark/70 hover:text-dark"
							onclick={() => (replyTo = null)}
						>
							cancel
						</button>
					</div>
				{/if}
				<div class="ml-auto mb-4 w-fit duo-btn duo-blue">
					<button
						class="fill-white"
						type="button"
						disabled={comments.sending || comments.current.length < 1}
						onclick={async () => {
							const headers =
								$user !== undefined
									? {
											Authorization: auth(),
											'Content-Type': 'application/json'
										}
									: {
											'Content-Type': 'application/json'
										};

							comments.sending = true;

							const res = await fetch(`/api/posts/id/${postId}/comments/new`, {
								method: 'PUT',
								headers,
								body: JSON.stringify({
									content: comments.current,
									parent_id: replyTo?.id ?? null
								})
							});
							if (res.ok) {
								const data = await res.json();
								const userData =
									$user !== undefined
										? {
												display_name: $user.displayName,
												username: $user.username,
												user_role: $user.role
											}
										: {};
								const newComment = {
									id: data.comment_id,
									avatar_url: userAvatarUrl,
									content: comments.current,
									parent_id: replyTo?.id ?? null,
									direct_reply_count: 0,
									created_at: undefined,
									...userData
								};
								comments.current = '';
								resizeTextarea(textarea);
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

									const thread = ensureReplyThread(
										parentId,
										parentComment?.direct_reply_count ?? 1
									);
									thread.total = Math.max(thread.total ?? 0, 1);
									updateReplyItems(parentId, [newComment], true);
									thread.endReached = thread.items.length >= (thread.total ?? 0);
								}

								replyTo = null;
								resetMentionState();
								resetCommandState();
							}
							comments.sending = false;
						}}
					>
						<svg class="w-6 -scale-x-100 -translate-y-0.5 inline-block" viewbox="0 0 24 24">
							<g>
								<path
									fill-rule="evenodd"
									clip-rule="evenodd"
									d="M3.3572 3.23397C3.66645 2.97447 4.1014 2.92638 4.45988 3.11204L20.7851 11.567C21.1426 11.7522 21.3542 12.1337 21.322 12.5351C21.2898 12.9364 21.02 13.2793 20.6375 13.405L13.7827 15.6586L10.373 22.0179C10.1828 22.3728 9.79826 22.5789 9.39743 22.541C8.9966 22.503 8.65762 22.2284 8.53735 21.8441L3.04564 4.29872C2.92505 3.91345 3.04794 3.49346 3.3572 3.23397ZM5.67123 5.99173L9.73507 18.9752L12.2091 14.361C12.3304 14.1347 12.5341 13.9637 12.7781 13.8835L17.7518 12.2484L5.67123 5.99173Z"
								></path>
							</g>
						</svg>
						Send
					</button>
				</div>
			</div>
		</div>
	</div>
	{#if comments.endReached && comments.roots.length === 0 && !comments.fetching}
		<div class="text-center py-8 text-dark/60">
			<p class="text-lg">
				No comments yet be the first! <span class="whitespace-nowrap">(⌒▽⌒)☆</span>
			</p>
		</div>
	{/if}
	<CommentThread
		comments={comments.roots}
		{postAuthorUsername}
		{replyThreads}
		onReply={handleReply}
		onToggleReplies={toggleReplies}
		onLoadMoreReplies={loadMoreReplies}
	/>
	<div class="mt-8 mx-auto w-fit duo-btn duo-blue">
		<button disabled={comments.endReached || comments.fetching} onclick={fetchComments}>
			{comments.endReached ? 'No more to read ٩(｡•́‿•̀｡)۶' : 'Read more'}
		</button>
	</div>
</section>

<style lang="postcss">
	@reference "../../../app.css";

	.comment-input {
		overflow-wrap: anywhere;
		word-break: break-word;
		white-space: pre-wrap;
	}
</style>
