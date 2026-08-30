import { getGsap } from '$lib/gsap.js';
import { authState } from '$lib/auth/user.svelte.js';
import { resizeTextarea } from '$lib/dom/auto-resize';
import { loadHighlightJs } from '$lib/custom-rules/plugins/code-highlight-lazy.js';
import { createCommentApi } from '../data/comment-api.js';
import {
	createCommentFeatureState,
	createEmptyState,
	createMentionState,
	createCommandState,
	createDrawerState,
	createUiState,
	resetObject
} from '../model/state.js';
import {
	createCommentNodeView,
	createSuggestionPanelState,
	normalizeAvatarUrl
} from '../model/view.js';
import { createCommentSyntaxEngine } from '../syntax/engine.js';
import { createMentionPlugin } from '../syntax/plugins/mention.js';
import { createKaomojiCommandPlugin } from '../syntax/plugins/command-kaomoji.js';
import { createGifCommandPlugin, buildGifMarkdown } from '../syntax/plugins/command-gif.js';
import { createToolbarActions } from '../syntax/plugins/toolbar.js';
import { createThreadController } from '../controllers/thread-controller.js';
import { createComposerController } from '../controllers/composer-controller.js';
import CommentGifPopover from '$lib/components/post/popovers/CommentGifPopover.svelte';
import CommentKaomojiPopover from '$lib/components/post/popovers/CommentKaomojiPopover.svelte';

const mentionMinChars = 3;
const mentionDebounceMs = 250;
const commandDebounceMs = 250;

function getTextareaAnchorOffsetY(node, anchorIndex) {
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
}

export function createCommentSectionViewModel({
	state = createCommentFeatureState(),
	getUserAvatarUrl,
	getGuestIdentity,
	onCommentPosted
}) {
	const api = createCommentApi();
	const mentionDictionary = {};
	const mentionProfileCache = new Map();
	let mentionDebounceTimer;
	let commandDebounceTimer;
	let popoverAnchorRaf;
	let activeMentionMatch = null;
	let activeCommandMatch = null;

	const engine = createCommentSyntaxEngine({
		mentionDictionary,
		plugins: [
			createMentionPlugin({
				searchProfiles: (query) => api.searchMentionProfiles(query)
			}),
			createKaomojiCommandPlugin({
				searchKaomojis: (query) => api.searchCommandKaomojis(query),
				PopoverComponent: CommentKaomojiPopover
			}),
			createGifCommandPlugin({
				searchGifs: (query) => api.searchCommandGifs(query),
				PopoverComponent: CommentGifPopover
			})
		]
	});
	engine.toolbar = createToolbarActions();

	const threadController = createThreadController({
		commentsState: state.comments,
		replyThreads: state.replyThreads
	});

	const getTextarea = () => state.textarea;
	const getComposerSurface = () => state.composerSurface;
	const getPopoverSurface = () => state.popoverSurface ?? state.composerSurface;
	const getStart = () => state.start;

	const composerController = createComposerController({
		state,
		getTextarea,
		engine,
		onAfterInput: () => schedulePopoverAnchor()
	});

	function schedulePopoverAnchor() {
		if (popoverAnchorRaf) {
			cancelAnimationFrame(popoverAnchorRaf);
		}

		popoverAnchorRaf = requestAnimationFrame(() => {
			const textarea = getTextarea();
			const composerSurface = getComposerSurface();
			const popoverSurface = getPopoverSurface();
			if (!textarea || !composerSurface || !popoverSurface) {
				state.ui.popoverTop = null;
				return;
			}

			const anchorIndex = state.commandState.open
				? (activeCommandMatch?.context?.start ?? state.commandState.start)
				: state.mentionState.open
					? (activeMentionMatch?.context?.start ?? state.mentionState.start)
					: -1;

			if (anchorIndex < 0) {
				state.ui.popoverTop = null;
				return;
			}

			const textareaRect = textarea.getBoundingClientRect();
			const containerRect = popoverSurface.getBoundingClientRect();
			const anchorOffset = getTextareaAnchorOffsetY(textarea, anchorIndex);
			if (anchorOffset == null) {
				state.ui.popoverTop = null;
				return;
			}

			state.ui.popoverTop = Math.max(0, textareaRect.top - containerRect.top + anchorOffset);
		});
	}

	function clearMentionState() {
		Object.assign(state.mentionState, createMentionState());
		activeMentionMatch = null;
		state.ui.popoverTop = null;
	}

	function clearCommandState() {
		Object.assign(state.commandState, createCommandState());
		activeCommandMatch = null;
		state.ui.popoverTop = null;
	}

	function clearTransientUi() {
		clearMentionState();
		clearCommandState();
	}

	async function ensureMentionProfile(username) {
		if (mentionProfileCache.has(username)) {
			const cached = mentionProfileCache.get(username);
			if (cached) {
				mentionDictionary[username] = cached;
			}
			return cached;
		}

		const profile = await api.searchMentionProfiles(username, 1).then((items) => items[0] ?? null);
		mentionProfileCache.set(username, profile);
		if (profile) {
			mentionDictionary[username] = profile;
		}
		return profile;
	}

	function extractMentionUsernames(value) {
		if (!value) return [];
		const usernames = new Set();
		const scan = /(^|[\s(>])@([A-Za-z0-9_-]{3,32})\b/g;
		let match;
		while ((match = scan.exec(value)) !== null) {
			if (match[2]) usernames.add(match[2]);
		}
		return [...usernames];
	}

	async function hydrateMentionDictionary(contents) {
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
	}

	async function renderComments(items) {
		await loadHighlightJs();
		await hydrateMentionDictionary(items.map((comment) => comment.content));
		return items.map((comment) => ({
			...comment,
			content: engine.markdown.render(comment.content)
		}));
	}

	async function fetchComments() {
		if (state.comments.fetching) return;
		state.comments.fetching = true;
		const page = await api.fetchRoots(
			state.postId,
			state.comments.lastId === 0 ? null : state.comments.lastId
		);
		page.items = await renderComments(page.items);
		threadController.updateRoots(page);
		state.comments.fetching = false;
	}

	async function loadMoreReplies(parentId) {
		const thread = threadController.ensureReplyThread(parentId);
		if (thread.fetching || thread.endReached) return;

		thread.fetching = true;
		const page = await api.fetchReplies(
			state.postId,
			parentId,
			thread.lastId === 0 ? null : thread.lastId
		);
		page.items = await renderComments(page.items);
		threadController.updateReplies(parentId, page);
		thread.fetching = false;
	}

	async function toggleReplies(comment) {
		const total = comment.direct_reply_count ?? 0;
		const thread = threadController.ensureReplyThread(comment.id, total);
		thread.expanded = !thread.expanded;
		if (thread.expanded && thread.items.length === 0 && total > 0) {
			thread.endReached = false;
			await loadMoreReplies(comment.id);
		}
	}

	function handleReply(comment, rootId) {
		state.composer.replyTo = { ...comment, rootId };
		getTextarea()?.focus();
	}

	function toggleMarkdownHelp() {
		state.ui.showMarkdownHelp = !state.ui.showMarkdownHelp;
		if (state.ui.showMarkdownHelp) {
			state.drawers.showGifSearch = false;
			state.drawers.showKaomojiSearch = false;
			clearCommandState();
		}
	}

	function toggleGifDrawer() {
		state.drawers.showGifSearch = !state.drawers.showGifSearch;
		state.drawers.showKaomojiSearch = false;
		state.ui.showMarkdownHelp = false;
		clearCommandState();
	}

	function resetKaomojiDrawerState() {
		state.drawers.kaomojiResults = [];
		state.drawers.kaomojiSuggestions = [];
		state.drawers.kaomojiPage = 1;
		state.drawers.kaomojiTotal = 0;
		state.drawers.kaomojiError = null;
	}

	function toggleKaomojiDrawer() {
		state.drawers.showKaomojiSearch = !state.drawers.showKaomojiSearch;
		state.drawers.showGifSearch = false;
		state.ui.showMarkdownHelp = false;
		clearCommandState();
		if (!state.drawers.showKaomojiSearch) {
			resetKaomojiDrawerState();
		}
	}

	async function fetchGifs(reset = false) {
		if (state.drawers.gifLoading) return;
		state.drawers.gifLoading = true;
		state.drawers.gifError = null;
		const offset = reset ? 0 : state.drawers.gifOffset;
		if (reset) {
			state.drawers.gifResults = [];
			state.drawers.gifOffset = 0;
		}

		const result = await api.fetchDrawerGifs(state.drawers.gifQuery, offset, reset);
		if (result.error) {
			state.drawers.gifError = result.error;
		} else if (reset) {
			state.drawers.gifResults = result.items;
			state.drawers.gifOffset = result.nextOffset;
		} else {
			state.drawers.gifResults = [...state.drawers.gifResults, ...result.items];
			state.drawers.gifOffset = result.nextOffset;
		}
		state.drawers.gifLoading = false;
	}

	async function fetchKaomojis(reset = false) {
		if (state.drawers.kaomojiLoading) return;
		const mood = state.drawers.kaomojiMood.trim().toLowerCase();
		if (!mood) {
			state.drawers.kaomojiError = null;
			return;
		}

		state.drawers.kaomojiLoading = true;
		state.drawers.kaomojiError = null;
		const page = reset ? 1 : state.drawers.kaomojiPage;
		if (reset) {
			resetKaomojiDrawerState();
		}

		const result = await api.fetchDrawerKaomojis(mood, page);
		state.drawers.kaomojiSuggestions = result.suggestions;
		state.drawers.kaomojiTotal = result.total;
		state.drawers.kaomojiPage = page + 1;
		state.drawers.kaomojiError = result.error;
		if (reset) {
			state.drawers.kaomojiResults = result.items;
		} else {
			state.drawers.kaomojiResults = [...state.drawers.kaomojiResults, ...result.items];
		}
		state.drawers.kaomojiLoading = false;
	}

	function selectGif(gif) {
		const markdown = buildGifMarkdown(gif);
		if (!markdown) return;
		state.drawers.showGifSearch = false;
		composerController.insertAtCursor(markdown);
	}

	function selectKaomoji(kaomoji) {
		if (!kaomoji) return;
		state.drawers.showKaomojiSearch = false;
		composerController.insertAtCursor(`@@[${kaomoji}]@@ `);
	}

	function applyKaomojiMoodSuggestion(suggestion) {
		if (!suggestion) return;
		state.drawers.kaomojiMood = suggestion;
		fetchKaomojis(true);
	}

	async function refreshMentionSuggestions() {
		const textarea = getTextarea();
		if (!textarea) {
			clearMentionState();
			return;
		}

		const detected = engine.detectActive(
			'mention',
			state.comments.current,
			textarea.selectionStart ?? state.comments.current.length
		);
		if (!detected || detected.context.query.length < mentionMinChars) {
			clearMentionState();
			return;
		}
		activeMentionMatch = detected;

		state.mentionState.query = detected.context.query;
		state.mentionState.start = detected.context.start;
		state.mentionState.loading = true;
		const requestId = state.mentionState.requestId + 1;
		state.mentionState.requestId = requestId;

		const result = await detected.plugin.search(detected.context);
		if (requestId !== state.mentionState.requestId) return;
		state.mentionState.items = result.items;
		state.mentionState.items.forEach((profile) => {
			mentionProfileCache.set(profile.username, profile);
			mentionDictionary[profile.username] = profile;
		});
		state.mentionState.selected = 0;
		state.mentionState.open = state.mentionState.items.length > 0;
		state.mentionState.loading = false;
		schedulePopoverAnchor();
	}

	async function refreshCommandSuggestions() {
		const textarea = getTextarea();
		if (!textarea) {
			clearCommandState();
			return;
		}

		const detected = engine.detectActive(
			'command',
			state.comments.current,
			textarea.selectionStart ?? state.comments.current.length
		);
		if (!detected) {
			clearCommandState();
			return;
		}
		activeCommandMatch = detected;

		clearMentionState();
		state.commandState.loading = true;
		state.commandState.open = true;
		state.commandState.type = detected.plugin.key;
		state.commandState.meta = detected.plugin.meta;
		state.commandState.query = detected.context.query;
		state.commandState.start = detected.context.start;
		state.commandState.replaceEnd = detected.context.replaceEnd;
		state.commandState.selected = 0;
		state.commandState.error = null;
		const requestId = state.commandState.requestId + 1;
		state.commandState.requestId = requestId;

		const result = await detected.plugin.search(detected.context);
		if (requestId !== state.commandState.requestId) return;
		state.commandState.items = result.items;
		state.commandState.suggestions = result.suggestions;
		state.commandState.error = result.error;
		state.commandState.loading = false;
		state.commandState.open =
			result.items.length > 0 ||
			result.suggestions.length > 0 ||
			Boolean(result.error) ||
			detected.context.query.trim().length === 0;
		schedulePopoverAnchor();
	}

	function scheduleMentionSearch() {
		if (mentionDebounceTimer) clearTimeout(mentionDebounceTimer);
		mentionDebounceTimer = setTimeout(refreshMentionSuggestions, mentionDebounceMs);
	}

	function scheduleCommandSearch() {
		if (commandDebounceTimer) clearTimeout(commandDebounceTimer);
		commandDebounceTimer = setTimeout(refreshCommandSuggestions, commandDebounceMs);
	}

	function handleComposerInput(event) {
		if (event?.type === 'keyup') {
			const ignoredKeys = [
				'ArrowDown',
				'ArrowUp',
				'ArrowLeft',
				'ArrowRight',
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
	}

	function handleTextareaBlur(event) {
		setTimeout(() => {
			const activeEl = document.activeElement;
			if (activeEl && activeEl.closest('.comment-autocomplete-popover')) return;
			if (event.relatedTarget && event.relatedTarget.closest('.comment-autocomplete-popover'))
				return;
			clearTransientUi();
		}, 100);
	}

	function applySelectionResult(result) {
		if (!result) return;
		state.comments.current = result.value;
		composerController.syncLayout(result.selectionStart, result.selectionEnd);
	}

	function pickMention(profile) {
		if (!activeMentionMatch) return;
		const result = activeMentionMatch.plugin.apply(
			{
				value: state.comments.current,
				start: activeMentionMatch.context.start,
				caret: getTextarea()?.selectionStart ?? state.comments.current.length
			},
			profile
		);
		clearMentionState();
		applySelectionResult(result);
	}

	function pickCommandItem(item) {
		if (!activeCommandMatch || state.commandState.start < 0) return;
		const result = activeCommandMatch.plugin.apply(
			{
				value: state.comments.current,
				start: activeCommandMatch.context.start,
				replaceEnd: activeCommandMatch.context.replaceEnd
			},
			item
		);
		clearCommandState();
		applySelectionResult(result);
	}

	function applyKaomojiSuggestion(suggestion) {
		if (!activeCommandMatch || !activeCommandMatch.plugin.applySuggestion) return;
		const result = activeCommandMatch.plugin.applySuggestion(
			{
				value: state.comments.current,
				start: activeCommandMatch.context.start,
				replaceEnd: activeCommandMatch.context.replaceEnd
			},
			suggestion
		);
		applySelectionResult(result);
		refreshCommandSuggestions();
	}

	function handleTextareaKeydown(event) {
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
				composerController.wrapBold();
				return;
			}
			if (key === 'i') {
				event.preventDefault();
				composerController.wrapItalic();
				return;
			}
			if (key === 'e') {
				event.preventDefault();
				composerController.wrapCode();
				return;
			}
		}

		if (state.commandState.open) {
			const activeItems =
				state.commandState.items.length > 0
					? state.commandState.items
					: state.commandState.suggestions;
			if (event.key === 'ArrowDown' || event.key === 'ArrowRight') {
				event.preventDefault();
				if (activeItems.length > 0) {
					state.commandState.selected = (state.commandState.selected + 1) % activeItems.length;
				}
				return;
			}
			if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') {
				event.preventDefault();
				if (activeItems.length > 0) {
					state.commandState.selected =
						(state.commandState.selected - 1 + activeItems.length) % activeItems.length;
				}
				return;
			}
			if (event.key === 'Enter' || event.key === 'Tab') {
				if (state.commandState.items.length > 0) {
					event.preventDefault();
					pickCommandItem(state.commandState.items[state.commandState.selected]);
					return;
				}
				if (state.commandState.suggestions.length > 0) {
					event.preventDefault();
					applyKaomojiSuggestion(
						state.commandState.suggestions[state.commandState.selected] ??
							state.commandState.suggestions[0]
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

		if (!state.mentionState.open || state.mentionState.items.length === 0) return;
		if (event.key === 'ArrowDown') {
			event.preventDefault();
			state.mentionState.selected =
				(state.mentionState.selected + 1) % state.mentionState.items.length;
			return;
		}
		if (event.key === 'ArrowUp') {
			event.preventDefault();
			state.mentionState.selected =
				(state.mentionState.selected - 1 + state.mentionState.items.length) %
				state.mentionState.items.length;
			return;
		}
		if (event.key === 'Enter' || event.key === 'Tab') {
			event.preventDefault();
			pickMention(state.mentionState.items[state.mentionState.selected]);
			return;
		}
		if (event.key === 'Escape') {
			event.preventDefault();
			clearMentionState();
		}
	}

	async function submitComment() {
		if (state.comments.sending || state.comments.current.length < 1) return;
		state.comments.commentError = '';
		state.comments.sending = true;

		try {
			const response = await api.submitComment({
				postId: state.postId,
				content: state.comments.current,
				parentId: state.composer.replyTo?.id ?? null,
				guestIdentity: getGuestIdentity()
			});

			if (!response.ok) {
				state.comments.commentError = response.error;
				return;
			}

			onCommentPosted?.();
			const currentUser = authState.user;
			const guestIdentity = getGuestIdentity();
			const isAlias = currentUser !== undefined && guestIdentity;
			const userData =
				currentUser !== undefined && !isAlias
					? {
							display_name: currentUser.displayName,
							username: currentUser.username,
							user_role: currentUser.role
						}
					: {};

			const optimisticComment = createCommentNodeView({
				id: response.commentId,
				avatar_url: normalizeAvatarUrl(getUserAvatarUrl()),
				content: state.comments.current,
				parent_id: state.composer.replyTo?.id ?? null,
				direct_reply_count: 0,
				created_at: undefined,
				guest_identity: guestIdentity ?? undefined,
				...userData
			});

			await hydrateMentionDictionary([optimisticComment.content]);
			optimisticComment.content = engine.markdown.render(optimisticComment.content);

			if (optimisticComment.parent_id != null) {
				await threadController.expandReplyChain(optimisticComment.parent_id, loadMoreReplies);
			}
			threadController.insertOptimisticComment(optimisticComment);

			state.comments.current = '';
			const textarea = getTextarea();
			if (textarea) {
				resizeTextarea(textarea);
			}
			state.composer.replyTo = null;
			clearTransientUi();
		} finally {
			state.comments.sending = false;
		}
	}

	function closeMarkdownHelp() {
		state.ui.showMarkdownHelp = false;
	}

	function resetForPost(postId) {
		state.postId = postId;
		Object.assign(state.comments, createEmptyState().comments);
		resetObject(state.replyThreads, {});
		Object.assign(state.composer, createEmptyState().composer);
		Object.assign(state.mentionState, createMentionState());
		Object.assign(state.commandState, createCommandState());
		Object.assign(state.drawers, createDrawerState());
		Object.assign(state.ui, createUiState());
		api.clearAll();
	}

	function handlePostChange(postId) {
		resetForPost(postId);
		const start = getStart();
		if (!start) return () => {};

		let cancelled = false;
		let onScrolled;
		let triggerInstance;

		// GSAP arrives asynchronously, so the scroll trigger attaches a tick
		// later; the cleanup must also cover the not-yet-created tween.
		getGsap().then(({ gsap }) => {
			if (cancelled) return;
			onScrolled = gsap.to(start, {
				scrollTrigger: {
					trigger: start,
					once: true,
					start: 'bottom bottom',
					onEnter: fetchComments
				}
			});
			triggerInstance = onScrolled.scrollTrigger;
		});

		return () => {
			cancelled = true;
			triggerInstance?.kill();
			onScrolled?.kill();
		};
	}

	return {
		state,
		md: engine.markdown,
		get suggestionPanel() {
			const panel = createSuggestionPanelState(state.commandState, state.mentionState);
			panel.top = state.ui.popoverTop;
			return panel;
		},
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
		insertAtCursor: (text) => composerController.insertAtCursor(text),
		insertHeader: () => composerController.insertHeader(),
		wrapBold: () => composerController.wrapBold(),
		wrapItalic: () => composerController.wrapItalic(),
		wrapCode: () => composerController.wrapCode(),
		pickCommandItem,
		applyKaomojiSuggestion,
		pickMention,
		schedulePopoverAnchor,
		fetchGifs,
		fetchKaomojis,
		selectGif,
		selectKaomoji,
		applyKaomojiMoodSuggestion
	};
}

export function createCommentSectionRuntime(
	state,
	getUserAvatarUrl,
	getGuestIdentity,
	onCommentPosted
) {
	return createCommentSectionViewModel({
		state,
		getUserAvatarUrl,
		getGuestIdentity,
		onCommentPosted
	});
}
