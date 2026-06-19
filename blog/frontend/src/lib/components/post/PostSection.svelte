<script>
	import { authState } from '$lib/auth/user.svelte.js';
	import { win } from '$lib/dom/windows.svelte.js';
	import { onDestroy, onMount, untrack } from 'svelte';
	import PBody from '../shell/PBody.svelte';
	import Portal from '../shell/Portal.svelte';
	import ContentTable from './ContentTable.svelte';
	import Post from './Post.svelte';
	import { browser } from '$app/environment';
	import { sendLike } from '$lib/utils/post';

	import { page } from '$app/state';
	import Copy from '../svgs/Copy.svelte';
	import X from '../svgs/X.svelte';
	import Linkedin from '../svgs/Linkedin.svelte';

	let {
		id,
		title,
		tags,
		date,
		updateTime,
		content,
		author,
		series,
		liked: initialLiked,
		editHref,
		relatedPosts = []
	} = $props();

	let copyDone = $state(false);

	function copyLink() {
		navigator.clipboard.writeText(page.url.href).then(() => {
			copyDone = true;
			setTimeout(() => (copyDone = false), 2000);
		});
	}

	let liked = $state(untrack(() => initialLiked));
	$effect(() => {
		liked = initialLiked;
	});
	let headers = $state([]);

	let toggled = $state(false);
	let pushingLike = $state(false);

	$effect(() => {
		if (win.isXl) toggled = false;
	});

	function onHashChange() {
		toggled = false;
	}

	onMount(() => {
		if (!browser) return;

		window.addEventListener('hashchange', onHashChange);
	});

	onDestroy(() => {
		if (!browser) return;

		window.removeEventListener('hashchange', onHashChange);
	});
</script>

<section class="flex not-xl:flex-col h-fit max-w-full">
	<div
		class="flex grow flex-col bg-white p-4 gap-4 rounded-xl not-xl:rounded-b-none xl:rounded-tr-none"
	>
		<div class="space-y-2 text-base">
			<div class="flex gap-4">
				<div class="*:inline">
					<button
						class="xl:hidden translate-y-1"
						title="table-of-contents-on"
						onclick={() => (toggled = !toggled)}
					>
						<svg class="w-7 lg:w-8 h-7 lg:h-8 fill-dark" viewBox="0 0 24 24">
							<path
								d="M6.25 7C6.25 7.69036 5.69036 8.25 5 8.25C4.30964 8.25 3.75 7.69036 3.75 7C3.75 6.30964 4.30964 5.75 5 5.75C5.69036 5.75 6.25 6.30964 6.25 7ZM9 6C8.44771 6 8 6.44772 8 7C8 7.55228 8.44771 8 9 8H19C19.5523 8 20 7.55228 20 7C20 6.44772 19.5523 6 19 6H9ZM9 11C8.44771 11 8 11.4477 8 12C8 12.5523 8.44771 13 9 13H19C19.5523 13 20 12.5523 20 12C20 11.4477 19.5523 11 19 11H9ZM9 16C8.44771 16 8 16.4477 8 17C8 17.5523 8.44771 18 9 18H19C19.5523 18 20 17.5523 20 17C20 16.4477 19.5523 16 19 16H9ZM5 13.25C5.69036 13.25 6.25 12.6904 6.25 12C6.25 11.3096 5.69036 10.75 5 10.75C4.30964 10.75 3.75 11.3096 3.75 12C3.75 12.6904 4.30964 13.25 5 13.25ZM5 18.25C5.69036 18.25 6.25 17.6904 6.25 17C6.25 16.3096 5.69036 15.75 5 15.75C4.30964 15.75 3.75 16.3096 3.75 17C3.75 17.6904 4.30964 18.25 5 18.25Z"
							></path>
						</svg>
					</button>
					<h1 class="text-2xl lg:text-4xl">
						{title}
					</h1>
				</div>
				{#if id && authState.user?.username === author.username}
					<div class="h-fit duo-btn duo-green">
						<a href={editHref ?? `/dashboard/posts/id/${id}`}>Edit</a>
					</div>
				{/if}
			</div>
			<div class="inline gap-2 text-dark/60">
				{#if tags?.length > 0}
					<ul class="inline text-dark *:inline space-x-1">
						{#each tags as tag}
							<li
								class="rounded-full px-1 border-2 border-primary *:no-underline! has-hover:bg-primary duration-100 transition-colors"
							>
								<a
									class="inline-block text-primary hover:text-white hover:*:text-white duration-100 transition-colors"
									href={`/tags/${tag}`}
								>
									<span class="text-gray-300">#</span>
									{tag}
								</a>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
			<div class="flex items-center gap-2 py-4">
				<hr class="xl:hidden grow border" />
				<span class="text-nowrap">{date} (updated: {updateTime})</span>
				<hr class="grow border" />
			</div>
		</div>
		<Post {content} bind:headers />

		{#if liked !== undefined}
			<div class="flex flex-wrap items-center gap-2">
				<div class="w-fit duo-btn duo-blue">
					<button
						disabled={liked || pushingLike}
						onclick={async () => {
							pushingLike = true;
							const res = await fetch(`/api/posts/id/${id}/like`, {
								method: 'POST'
							});
							if (res.ok) {
								sendLike(id);
								liked = true;
								pushingLike = false;
							}
						}}
					>
						{pushingLike ? 'Upvote?' : liked ? 'Upvoted!' : 'Upvote?'}
					</button>
				</div>
				<div class="ml-auto flex flex-wrap items-center text-sm font-bold gap-2">
					<span class="font-normal text-dark/60">Share:</span>
					<button
						class="w-fit px-2 h-8 rounded-lg border-2 border-accent-green text-accent-green bg-accent-green-light-2/50 hover:bg-accent-green hover:text-white transition-colors"
						onclick={copyLink}
						title="Copy link"
					>
						<Copy class="inline w-4 h-4" />
						{copyDone ? 'Copied!' : 'Copy link'}
					</button>
					<a
						class="w-fit px-3 h-8 inline-flex gap-1 items-center rounded-lg border-2 border-black text-black bg-white hover:bg-black hover:text-white transition-colors hover:no-underline!"
						href={`https://x.com/intent/post?url=${encodeURIComponent(page.url.href)}&text=${encodeURIComponent(title)}`}
						target="_blank"
						rel="noopener noreferrer"
						title="Share on X"
					>
						<X class="inline w-4 h-4" />
						/ Twitter
					</a>
					<a
						class="w-fit px-3 h-8 inline-flex gap-0.5 items-center rounded-lg border-2 border-accent-blue bg-accent-blue-light-2/20 text-accent-blue hover:bg-accent-blue hover:text-white transition-colors hover:no-underline!"
						href={`https://www.linkedin.com/sharing/share-offsite/?url=${encodeURIComponent(page.url.href)}`}
						target="_blank"
						rel="noopener noreferrer"
						title="Share on LinkedIn"
					>
						Linked<Linkedin class="inline w-3 h-3 -translate-y-px" />
					</a>
				</div>
			</div>
		{/if}

		{#if series}
			{@const {
				title,
				slug,
				cover_url: url,
				previous_post: previousPost,
				next_post: nextPost
			} = series}
			<div class="w-full space-y-2 bg-primary/30 p-2 rounded-lg">
				<h2 class="text-center">
					This post is in the series <a class="font-bold" href={`/series/${slug}`}>{title}</a>
				</h2>
				<div
					class="grid grid-cols-1 lg:grid-cols-2 gap-2 *:relative *:h-20 *:rounded-lg *:overflow-hidden"
				>
					{#if previousPost}
						{@const { title, slug, cover_url: url } = previousPost}
						<div class="p-2 flex flex-row-reverse hover:[&>.arrow]:-translate-x-4 gap-2">
							<a
								class="arrow absolute z-9 top-0 h-full left-20 -right-20 bg-primary/30 transition-transform duration-200"
								href={`/posts/${slug}`}
								title="previous-post"
							>
								<svg
									class="absolute right-full top-1/2 -translate-y-1/2 h-20 w-20 fill-primary/30"
									viewBox="0 0 10 10"
								>
									<polygon style="stroke-linejoin: round;" points="10 0 10 10 3 5" />
								</svg>
							</a>
							<a class="relative z-10" href={`/posts/${slug}`}>
								<img
									class="min-w-16 w-16 h-16 object-cover rounded-lg"
									src={url}
									alt="left-post-cover"
								/>
							</a>
							<div class="relative z-10 grow my-auto pointer-events-none">
								<div class="w-full max-w-[calc(100%-4rem)] my-auto ml-auto text-sm text-right">
									<span class="underline">Previous post</span>
									<a class="pointer-events-auto" href={`/posts/${slug}`}>
										<h2 class="font-bold line-clamp-2">
											{title}
										</h2>
									</a>
								</div>
							</div>
						</div>
					{:else}
						<div class="grid text-center rounded-lg bg-primary/30">
							<span class="my-auto">No earlier posts</span>
						</div>
					{/if}
					{#if nextPost}
						{@const { title, slug, cover_url: url } = nextPost}
						<div class="p-2 flex flex-row hover:[&>.arrow]:translate-x-4 gap-2">
							<a
								class="arrow absolute z-9 top-0 h-full -left-20 right-20 bg-primary/30 transition-transform duration-200"
								href={`/posts/${slug}`}
								title="previous-post"
							>
								<svg
									class="absolute left-full top-1/2 -translate-y-1/2 h-20 w-20 fill-primary/30"
									viewBox="0 0 10 10"
								>
									<polygon style="stroke-linejoin: round;" points="0 0 0 10 7 5" />
								</svg>
							</a>
							<a class="relative z-10" href={`/posts/${slug}`}>
								<img
									class="min-w-16 w-16 h-16 object-cover rounded-lg"
									src={url}
									alt="left-post-cover"
								/>
							</a>
							<div class="relative z-10 grow my-auto pointer-events-none">
								<div class="w-full max-w-[calc(100%-4rem)] my-auto mr-auto text-sm text-left">
									<span class="underline">Next post</span>
									<a class="pointer-events-auto" href={`/posts/${slug}`}>
										<h2 class="font-bold line-clamp-2">
											{title}
										</h2>
									</a>
								</div>
							</div>
						</div>
					{:else}
						<div class="grid text-center rounded-lg bg-primary/30">
							<span class="my-auto">No newer posts</span>
						</div>
					{/if}
				</div>
			</div>
		{/if}

		{#if relatedPosts.length > 0}
			<div class="w-full space-y-2">
				<h2 class="text-lg font-bold">You might also like</h2>
				<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
					{#each relatedPosts as post}
						<a
							class="flex gap-3 items-center p-2 rounded-lg border-2 border-primary/30 hover:border-primary hover:bg-primary/10 transition-colors"
							href={`/posts/${post.slug}`}
						>
							{#if post.cover_url}
								<img
									class="w-14 h-14 object-cover rounded-lg shrink-0"
									src={post.cover_url}
									alt={post.title}
								/>
							{/if}
							<span class="text-sm font-medium line-clamp-3">{post.title}</span>
						</a>
					{/each}
				</div>
			</div>
		{/if}
	</div>
	<div class="flex flex-col h-auto min-w-60">
		<div class="w-full *:full bg-white rounded-xl not-xl:rounded-t-none xl:rounded-l-none">
			<div class="xl:hidden p-4">
				<hr class="border" />
			</div>
			<span class="inline-block pl-4 pt-4">Written by:</span>
			<div class="flex flex-col gap-2 p-4 pt-2 text-dark">
				<div class="flex items-center gap-2 bg-secondary/60 p-2 rounded-lg">
					<div class="w-fit h-fit bg-radial from-white to-secondary rounded-full overflow-hidden">
						<img
							class="min-w-16 w-16 h-16 object-contain"
							src={author.avatarUrl ?? '/missing.png'}
							alt="author-avatar"
						/>
					</div>
					<div class="flex flex-col">
						<a class="font-semibold text-dark/80 text-nowrap" href={`/profiles/${author.username}`}>
							{author.displayName}
						</a>
						<span>{author.username}</span>
					</div>
				</div>
			</div>
			{#if series}
				{@const { title, slug, cover_url: url } = series}
				<span class="inline-block pl-4 pt-4">In series:</span>
				<div class=" p-4">
					<div class="flex flex-col gap-2 bg-secondary/60 rounded-lg overflow-hidden">
						<a class="block w-fit mt-4 mb-2 mx-auto" href={`/series/${slug}`}>
							<img
								class="w-36 min-w-36 h-36 min-h-36 object-cover rounded-lg"
								src={url}
								alt="series-cover"
							/>
						</a>
						<a
							class="block no-underline! text-center text-white font-semibold p-2 bg-primary hover:brightness-110 transition-all duration-100"
							href={`/series/${slug}`}
						>
							<h2>{title}</h2>
						</a>
					</div>
				</div>
			{/if}
		</div>
		<svg class="not-xl:hidden w-4 fill-white" viewBox="0 0 12 12">
			<path d="M 0,0 L 12,0 A 12,12 0 0 0 0,12 Z" />
		</svg>
		<div class="relative grow">
			<div class="sticky top-32 ml-4 not-xl:hidden bg-white text-base rounded-xl py-2">
				<h2 class="text-center font-semibold text-lg pb-2">Table of contents</h2>
				<div class="max-h-[calc(100vh-12.25rem)] px-2 overflow-y-auto custom-scrollbar">
					<ContentTable {headers} />
				</div>
			</div>
		</div>
	</div>
</section>

{#if !win.isXl && toggled}
	<PBody>
		<button
			class="absolute top-0 left-0 full cursor-not-allowed!"
			title="overlay"
			onwheel={(e) => e.preventDefault()}
			onclick={() => (toggled = false)}
		></button>
		<div
			class="fixed top-[calc(50%+1.75rem)] left-1/2 -translate-x-1/2 -translate-y-1/2 text-dark bg-white rounded-xl py-4"
		>
			<h2 class="text-center font-semibold text-lg pb-2">Table of contents</h2>
			<div
				class="max-h-[calc(100vh-9rem)] overflow-y-scroll custom-scrollbar max-w-full w-max px-4"
			>
				<ContentTable {headers} />
			</div>
		</div>
	</PBody>
{/if}
