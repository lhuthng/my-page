<script>
	import { fade, fly } from 'svelte/transition';
	import Club from '$lib/components/svgs/Club.svelte';
	import Diamond from '$lib/components/svgs/Diamond.svelte';
	import Heart from '$lib/components/svgs/Heart.svelte';
	import Spade from '$lib/components/svgs/Spade.svelte';
	import Comment from './Comment.svelte';
	import CommentThread from './CommentThread.svelte';

	let {
		comments = [],
		postAuthorUsername = null,
		replyThreads = {},
		onReply = () => {},
		onToggleReplies = () => {},
		onLoadMoreReplies = () => {},
		depth = 0,
		rootId = null
	} = $props();

	const isAuthor = (comment) => Boolean(comment.username && comment.display_name);
	const isPostAuthor = (comment) =>
		Boolean(postAuthorUsername && comment.username === postAuthorUsername);

	const roleTooltip = (comment) => {
		if (comment.user_role === 'admin') return 'Admin! ꨄ︎';
		if (comment.user_role === 'moderator') return 'a mod!';
		if (comment.user_role === 'user') return 'user';
		return 'wanderer';
	};

	const replyCountLabel = (count) => {
		if (!count) return 'no replies';
		return `${count} ${count === 1 ? 'reply' : 'replies'}`;
	};

	const getThread = (comment) =>
		replyThreads[comment.id] ?? {
			expanded: false,
			fetching: false,
			endReached: true,
			items: [],
			total: comment.direct_reply_count ?? 0
		};

	const rootForComment = (comment) => rootId ?? comment.id;
</script>

<ul class="flex flex-col gap-4 w-full min-w-0">
	{#each comments as comment (comment.id)}
		{@const thread = getThread(comment)}
		{@const anonymous = isAuthor(comment)}
		{@const thisRootId = rootForComment(comment)}

		<li
			in:fly={{ y: 10, duration: 500 }}
			class={`rounded-xl min-w-0 ${depth === 0 ? 'bg-white/80' : 'bg-white/60'}`}
		>
			<div class="flex py-2 min-w-0">
				<div
					class="ml-2 min-w-10 lg:min-w-12 w-10 lg:w-12 h-10 lg:h-12 outline-primary outline-2 rounded-full shadow-md overflow-hidden"
				>
					{#if anonymous}
						<a class="full" href={`/profiles/${comment.username}`}>
							<img
								class="full object-cover"
								src={comment.avatar_url ?? '/anonymous.gif'}
								alt="comment-avatar"
							/>
						</a>
					{:else}
						<img
							class="full object-cover"
							src={comment.avatar_url ?? '/anonymous.gif'}
							alt="comment-avatar"
						/>
					{/if}
				</div>
				<div class="relative flex flex-col grow min-w-0">
					<div class="pl-2 -translate-y-2 min-w-0">
						<div class="relative max-w-full min-w-0">
							<div class="max-w-full min-w-0 p-2 bg-primary/20 rounded-2xl rounded-tl-md">
								<div class="flex items-center lg:text-base">
									{#if anonymous}
										<a class="font-semibold" href={`/profiles/${comment.username}`}>
											{comment.display_name}
										</a>
										{#if isPostAuthor(comment)}
											<span class="ml-1 italic select-none text-dark/60">(author)</span>
										{/if}
									{:else}
										<span class="font-normal select-none italic">Anonymous</span>
									{/if}
									<span
										class="*:w-8 hover:*:translate-x-1 *:transition-all *:duration-200 tooltip-container"
										data-tooltip={roleTooltip(comment)}
									>
										{#if comment.user_role === 'admin'}
											<Heart class="fill-accent-red" />
										{:else if comment.user_role === 'moderator'}
											<Diamond class="fill-accent-red" />
										{:else if comment.user_role === 'user'}
											<Club class="fill-dark" />
										{:else}
											<Spade class="fill-dark" />
										{/if}
									</span>
								</div>

								<Comment content={comment.content} />
							</div>
							<div
								class="absolute flex min-w-20 w-full justify-between gap-2 left-0 top-full text-sm"
							>
								<span class="pl-2">{comment.created_at ?? 'new'}</span>
								<button
									type="button"
									class="text-nowrap cursor-pointer hover:text-primary mr-2"
									onclick={() => onReply(comment, thisRootId)}
								>
									reply
								</button>
							</div>
						</div>
					</div>
				</div>
			</div>

			{#if (comment.direct_reply_count ?? 0) > 0}
				<div class="ml-6 lg:ml-16 mt-2 flex items-center gap-1.5 text-xs text-dark/70">
					<button
						type="button"
						class="cursor-pointer font-medium text-primary hover:text-accent-blue"
						onclick={() => onToggleReplies(comment)}
					>
						{thread.expanded
							? 'Hide replies'
							: `View ${replyCountLabel(comment.direct_reply_count)}`}
					</button>
				</div>
			{/if}

			{#if thread.expanded}
				<div
					class="relative ml-8 lg:ml-18 mt-2 pl-2"
					in:fly={{ y: 10, duration: 500 }}
					out:fade={{ duration: 200 }}
				>
					<div class="absolute -left-2 top-0 w-0.5 h-full bg-primary/35"></div>

					{#if depth === 0}
						<div class="mb-2 text-[10px] font-semibold uppercase tracking-wide text-primary/80">
							Reply thread
						</div>
					{/if}

					{#if thread.items.length === 0 && !thread.fetching}
						<p class="text-sm text-dark/70">No replies yet.</p>
					{:else if thread.items.length > 0}
						<CommentThread
							comments={thread.items}
							{postAuthorUsername}
							{replyThreads}
							{onReply}
							{onToggleReplies}
							{onLoadMoreReplies}
							depth={depth + 1}
							rootId={thisRootId}
						/>
					{/if}

					{#if !thread.endReached}
						<div class="mt-2 w-fit duo-btn duo-blue">
							<button
								type="button"
								disabled={thread.fetching}
								onclick={() => onLoadMoreReplies(comment.id)}
							>
								{thread.fetching ? 'Loading...' : 'Read more replies'}
							</button>
						</div>
					{/if}
				</div>
			{/if}
		</li>
	{/each}
</ul>
