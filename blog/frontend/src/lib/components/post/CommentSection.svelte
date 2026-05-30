<script>
	import { autoHResize } from '$lib/client/auto-resize';
	import { auth, user } from '$lib/client/user';
	import { gsap } from 'gsap';
	import { fade } from 'svelte/transition';
	import MarkdownIt from 'markdown-it';
	import Comment from './Comment.svelte';
	import CommentThread from './CommentThread.svelte';
	import { codeHighlightPlugin, mentionProfilePlugin } from '$lib/custom-rules';

	const rootLimit = 3;
	const replyLimit = 5;
	const mentionDictionary = {};
	const mentionProfileCache = new Map();
	const mentionProfileInFlight = new Map();
	const rootPageCache = new Map();
	const rootPageInFlight = new Map();
	const replyPageCache = new Map();
	const replyPageInFlight = new Map();
	const md = new MarkdownIt()
		.use(codeHighlightPlugin)
		.use(mentionProfilePlugin, { mentionDictionary });
	const mentionDebounceMs = 250;
	const mentionMinChars = 3;
	const mentionMaxProfiles = 5;
	const mentionRegex = /(^|[\s(>])@([A-Za-z0-9_-]{3,32})\b/g;

	let { postId } = $props();
	let last = -1;

	let userAvatarUrl = $derived($user?.avatarUrl ?? '/anonymous.webp');

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
	let tab = $state('write');
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
		if (!url) return '/anonymous.webp';
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

		mentionState.query = context.query;
		mentionState.start = context.start;
		mentionState.loading = true;
		const requestId = mentionState.requestId + 1;
		mentionState.requestId = requestId;

		try {
			const res = await fetch(
				`/api/users?term=${encodeURIComponent(context.query)}&size=${mentionMaxProfiles}&offset=0`
			);

			if (requestId !== mentionState.requestId) {
				return;
			}

			if (!res.ok) {
				resetMentionState();
				return;
			}

			const data = await res.json();
			mentionState.items = (data.users ?? []).map((profile) => ({
				...profile,
				avatar_url: normalizeAvatarUrl(profile.avatar_url)
			}));
			mentionState.items.forEach((profile) => {
				mentionProfileCache.set(profile.username, profile);
				mentionDictionary[profile.username] = profile;
			});
			mentionState.selected = 0;
			mentionState.open = mentionState.items.length > 0;
			mentionState.loading = false;
		} catch {
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

	const pickMention = (pickedUser) => {
		if (!textarea || mentionState.start < 0) return;

		const caret = textarea.selectionStart ?? comments.current.length;
		const left = comments.current.slice(0, mentionState.start + 1);
		const right = comments.current.slice(caret);
		const injected = `${left}${pickedUser.username} ${right}`;

		comments.current = injected;
		resetMentionState();

		requestAnimationFrame(() => {
			const nextCaret = left.length + pickedUser.username.length + 1;
			textarea.focus();
			textarea.setSelectionRange(nextCaret, nextCaret);
		});
	};

	const handleTextareaKeydown = (event) => {
		if (!mentionState.open || mentionState.items.length === 0) return;

		if (event.key === 'ArrowDown') {
			event.preventDefault();
			mentionState.selected = (mentionState.selected + 1) % mentionState.items.length;
			return;
		}

		if (event.key === 'ArrowUp') {
			event.preventDefault();
			mentionState.selected =
				(mentionState.selected - 1 + mentionState.items.length) % mentionState.items.length;
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
	let gifQuery = $state('');
	let gifResults = $state([]);
	let gifLoading = $state(false);
	let gifOffset = $state(0);
	let gifError = $state(null);

	const fetchGifs = async (reset = false) => {
		if (gifLoading) return;
		gifLoading = true;
		gifError = null;

		const currentOffset = reset ? 0 : gifOffset;
		if (reset) {
			gifResults = [];
			gifOffset = 0;
		}

		try {
			const url = `/api/gifs?q=${encodeURIComponent(gifQuery)}&offset=${currentOffset}`;
			const res = await fetch(url);
			if (!res.ok) {
				throw new Error('Failed to load GIFs');
			}
			const payload = await res.json();
			const newGifs = payload.data || [];

			if (reset) {
				gifResults = newGifs;
			} else {
				gifResults = [...gifResults, ...newGifs];
			}
			gifOffset = currentOffset + newGifs.length;
		} catch (err) {
			gifError = 'Could not load GIFs. Please check your connection.';
		} finally {
			gifLoading = false;
		}
	};

	const selectGif = (gif) => {
		if (!textarea) return;
		const gifUrl = gif.images.original.url;
		const title = gif.title || 'gif';
		const markdownImage = `![${title}](${gifUrl})`;

		const startPos = textarea.selectionStart;
		const endPos = textarea.selectionEnd;

		comments.current =
			textarea.value.slice(0, startPos) + markdownImage + textarea.value.slice(endPos);

		showGifSearch = false; // Close drawer after selection

		requestAnimationFrame(() => {
			const nextCursor = startPos + markdownImage.length;
			textarea.focus();
			textarea.setSelectionRange(nextCursor, nextCursor);
		});
	};

	$effect(() => {
		if (showGifSearch && gifResults.length === 0) {
			fetchGifs(true);
		}
	});
</script>

<section class="w-full xl:w-[calc(100%-15rem)] bg-white p-4 rounded-xl">
	<h4 class="text-lg lg:text-2xl">Join the discussion!</h4>
	<div class="flex flex-col gap-4" bind:this={start}>
		<hr class="border-t-3 border-dark mb-6" />
		<div class="flex gap-8">
			<div
				class="not-xxs:hidden min-w-12 max-w-12 lg:min-w-20 lg:max-w-20 h-12 lg:h-20 outline-primary outline-3 rounded-full overflow-hidden"
			>
				<img class="full object-cover" src={userAvatarUrl} alt="comment-posting-avatar" />
			</div>
			<div class="grow flex flex-col gap-4 relative">
				<svg
					class="not-xxs:hidden absolute fill-primary top-6 lg:top-10 -left-4 -translate-y-1/2 w-4 h-4"
					viewBox="0 0 24 24"
				>
					<polygon points="0,12 24,0 24,24" />
				</svg>
				<div class="w-full">
					<div
						class="relative text-base w-full bg-primary-20 border-primary border-2 border-b-0 rounded-t-xl"
					>
						{#if tab === 'preview'}
							<div class="w-full min-h-16 lg:min-h-20 overflow-hidden p-2">
								<Comment content={md.render(comments.current)} />
							</div>
						{:else if tab === 'write'}
							<textarea
								name="comment-input"
								class="block w-full min-h-16 lg:min-h-20 overflow-hidden outline-none resize-none p-2"
								bind:this={textarea}
								bind:value={comments.current}
								oninput={scheduleMentionSearch}
								onclick={scheduleMentionSearch}
								onkeyup={scheduleMentionSearch}
								onkeydown={handleTextareaKeydown}
								onblur={() => {
									setTimeout(resetMentionState, 100);
								}}
								{@attach autoHResize}
							></textarea>
						{/if}

						{#if showGifSearch && tab === 'write'}
							<div class="border-t-2 border-primary/20 bg-white/40 p-3 flex flex-col gap-3">
								<div class="flex gap-2 h-9">
									<input
										type="text"
										bind:value={gifQuery}
										placeholder="Search Giphy..."
										class="flex-1 bg-white border border-primary/30 rounded-xl px-2 py-1 text-base text-dark placeholder-dark/50 outline-none focus:border-primary focus:border-2 transition-colors"
										onkeydown={(e) => {
											if (e.key === 'Enter') {
												e.preventDefault();
												fetchGifs(true);
											}
										}}
									/>
									<div class="ml-auto w-fit duo-btn duo-blue">
										<button type="button" onclick={() => fetchGifs(true)}>Search</button>
									</div>
								</div>

								{#if gifError}
									<p class="text-xs text-accent-red font-medium">{gifError}</p>
								{/if}

								<div
									class="grid grid-cols-3 xs:grid-cols-4 sm:grid-cols-6 gap-2 max-h-120 overflow-y-auto pr-1 custom-scrollbar"
								>
									{#each gifResults as gif (gif.id)}
										<button
											type="button"
											class="group relative rounded-lg overflow-hidden aspect-square border border-primary/10 hover:border-primary transition-all bg-dark/5"
											onclick={() => selectGif(gif)}
										>
											<img
												src={gif.images.fixed_height.url}
												alt={gif.title}
												class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-200"
												loading="lazy"
											/>
											<div
												class="absolute inset-0 bg-primary/70 opacity-0 group-hover:opacity-100 flex items-center justify-center font-bold text-xl text-white transition-opacity duration-150"
											>
												Select
											</div>
										</button>
									{/each}

									{#if gifLoading}
										{#each Array(6) as _}
											<div class="animate-pulse bg-primary/10 rounded-lg aspect-square"></div>
										{/each}
									{/if}
								</div>

								{#if gifResults.length > 0 && !gifLoading}
									<button
										type="button"
										class="text-md font-semibold text-primary hover:text-dark mx-auto py-1"
										onclick={() => fetchGifs(false)}
									>
										Load More
									</button>
								{/if}
							</div>
						{/if}

						{#if mentionState.open}
							<ul
								class="absolute left-2 right-2 top-full z-30 mt-1 max-h-64 overflow-y-auto rounded-xl border-2 border-primary bg-white p-1 shadow-lg"
							>
								{#each mentionState.items as profile, index (profile.username)}
									<li>
										<button
											type="button"
											class="flex w-full items-center gap-3 rounded-lg px-2 py-1 text-left hover:bg-primary-20"
											class:bg-primary-20={index === mentionState.selected}
											onmousedown={(event) => {
												event.preventDefault();
												pickMention(profile);
											}}
										>
											<img
												class="h-8 w-8 rounded-full object-cover outline-2 outline-primary"
												src={profile.avatar_url ?? '/anonymous.webp'}
												alt={`${profile.display_name} avatar`}
											/>
											<div class="min-w-0 flex-1">
												<p class="truncate font-semibold">{profile.display_name}</p>
												<p class="truncate text-xs opacity-80">@{profile.username}</p>
											</div>
										</button>
									</li>
								{/each}
							</ul>
						{/if}
					</div>
					<div
						class="comment-editor flex not-xxs:flex-col justify-between min-h-8 bg-primary rounded-b-xl"
					>
						<div class="flex">
							<button
								class="w-fit h-8 px-2 bg-primary-20 border-2 border-primary border-t-0 rounded-b-xl"
								class:z-11={tab === 'write'}
								class:z-9={tab !== 'write'}
								class:opacity-100={tab === 'write'}
								class:opacity-90={tab !== 'write'}
								onclick={() => (tab = 'write')}
							>
								Write
							</button>
							<button
								class="w-fit h-8 px-2 -translate-x-0.5 bg-primary-20 border-2 border-primary border-t-0 rounded-b-xl"
								class:z-11={tab === 'preview'}
								class:z-9={tab !== 'preview'}
								class:opacity-100={tab === 'preview'}
								class:opacity-90={tab !== 'preview'}
								onclick={() => {
									tab = 'preview';
									showGifSearch = false;
								}}
							>
								Preview
							</button>
						</div>
						{#if tab === 'write'}
							<div
								class="flex ml-auto h-full my-auto *:bg-primary fill-white *:w-8 *:h-8 *:*:mx-auto *:hover:brightness-120 *:active:*:translate-y-0.5 gap-2 mr-2"
								in:fade={{ duration: 100 }}
							>
								<button
									title="Header"
									onclick={() => {
										if (!textarea) return;
										const start = textarea.selectionStart;
										const end = textarea.selectionEnd;
										comments.current =
											textarea.value.slice(0, start) + '# ' + textarea.value.slice(start);

										requestAnimationFrame(() => {
											textarea.setSelectionRange(start + 2, end + 2);
											textarea.focus();
										});
									}}
								>
									<svg class="w-4 h-4" viewBox="0 0 16 16">
										<path
											d="M3.75 2a.75.75 0 0 1 .75.75V7h7V2.75a.75.75 0 0 1 1.5 0v10.5a.75.75 0 0 1-1.5 0V8.5h-7v4.75a.75.75 0 0 1-1.5 0V2.75A.75.75 0 0 1 3.75 2Z"
										></path>
									</svg>
								</button>
								<button
									title="Bold"
									onclick={() => {
										if (!textarea) return;
										const start = textarea.selectionStart;
										const end = textarea.selectionEnd;
										comments.current =
											textarea.value.slice(0, start) +
											'**' +
											textarea.value.slice(start, end) +
											'**' +
											textarea.value.slice(end);
										requestAnimationFrame(() => {
											textarea.setSelectionRange(start + 2, end + 2);
											textarea.focus();
										});
									}}
								>
									<svg class="w-4 h-4" viewBox="0 0 16 16">
										<path
											d="M4 2h4.5a3.501 3.501 0 0 1 2.852 5.53A3.499 3.499 0 0 1 9.5 14H4a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Zm1 7v3h4.5a1.5 1.5 0 0 0 0-3Zm3.5-2a1.5 1.5 0 0 0 0-3H5v3Z"
										></path>
									</svg>
								</button>
								<button
									title="Italic"
									onclick={() => {
										if (!textarea) return;
										const start = textarea.selectionStart;
										const end = textarea.selectionEnd;
										comments.current =
											textarea.value.slice(0, start) +
											'_' +
											textarea.value.slice(start, end) +
											'_' +
											textarea.value.slice(end);
										requestAnimationFrame(() => {
											textarea.setSelectionRange(start + 1, end + 1);
											textarea.focus();
										});
									}}
								>
									<svg class="w-4 h-4" viewBox="0 0 16 16">
										<path
											d="M6 2.75A.75.75 0 0 1 6.75 2h6.5a.75.75 0 0 1 0 1.5h-2.505l-3.858 9H9.25a.75.75 0 0 1 0 1.5h-6.5a.75.75 0 0 1 0-1.5h2.505l3.858-9H6.75A.75.75 0 0 1 6 2.75Z"
										></path>
									</svg>
								</button>
								<button
									title="Code"
									onclick={() => {
										if (!textarea) return;
										const start = textarea.selectionStart;
										const end = textarea.selectionEnd;
										textarea.value =
											textarea.value.slice(0, start) +
											'`' +
											textarea.value.slice(start, end) +
											'`' +
											textarea.value.slice(end);
										requestAnimationFrame(() => {
											textarea.setSelectionRange(start + 1, end + 1);
											textarea.focus();
										});
									}}
								>
									<svg class="w-4 h-4" viewBox="0 0 16 16">
										<path
											d="m11.28 3.22 4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.749.749 0 0 1-1.275-.326.749.749 0 0 1 .215-.734L13.94 8l-3.72-3.72a.749.749 0 0 1 .326-1.275.749.749 0 0 1 .734.215Zm-6.56 0a.751.751 0 0 1 1.042.018.751.751 0 0 1 .018 1.042L2.06 8l3.72 3.72a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L.47 8.53a.75.75 0 0 1 0-1.06Z"
										></path>
									</svg>
								</button>
								<button
									title="Insert GIF"
									type="button"
									class:bg-primary-20={showGifSearch}
									onclick={() => {
										showGifSearch = !showGifSearch;
									}}
								>
									<svg class="w-4 h-4" viewBox="0 0 40 40">
										<path
											d="M28.75,11.88V8.94H25.53V6H8.73V34H31.27V11.88Zm-16.94,19V9.08H23v5.46h5.18V30.92Z"
										/>
									</svg>
								</button>
							</div>
						{/if}
					</div>
				</div>
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
				<div class="ml-auto w-fit duo-btn duo-blue">
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

	.comment-editor {
		@apply relative *:relative;
		&::before {
			@apply pointer-events-none absolute! top-0 left-0 z-10 h-full w-full rounded-b-xl border-2 border-primary content-[''];
		}
	}
</style>
