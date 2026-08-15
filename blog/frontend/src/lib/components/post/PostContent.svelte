<script>
	import { authState } from '$lib/auth/user.svelte.js';
	import BackButton from '../ui/BackButton.svelte';
	import Post from './Post.svelte';

	// The "content part" of a post/project page: title row, optional
	// metadata (tags, date), and the rendered body. This is what the editor's
	// lightweight split-pane preview renders (title + body only), and the full
	// public page composes it inside the rest of its chrome via PostSection.
	let {
		id = null,
		title,
		tags = [],
		date = '',
		updateTime = '',
		content,
		author = null,
		editHref = null,
		hideBackButton = false,
		headers = $bindable(),
		tocToggle = null
	} = $props();
</script>

<div class="space-y-2 text-base">
	{#if !hideBackButton}
		<BackButton href="/posts" text="All posts" />
	{/if}
	<div class="flex gap-4">
		<div class="*:inline">
			{@render tocToggle?.()}
			<h1 class="text-2xl lg:text-4xl">
				{title}
			</h1>
		</div>
		{#if id && author?.username === authState.user?.username}
			<div class="h-fit duo-btn duo-green">
				<a href={editHref ?? `/dashboard/posts/id/${id}`}>Edit</a>
			</div>
		{/if}
	</div>
	{#if tags?.length > 0}
		<div class="inline gap-2 text-dark/60">
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
		</div>
	{/if}
	{#if date}
		<div class="flex items-center gap-2 py-4">
			<hr class="xl:hidden grow border" />
			<span class="text-nowrap">{date}{updateTime ? ` (updated: ${updateTime})` : ''}</span>
			<hr class="grow border" />
		</div>
	{/if}
	<Post {content} bind:headers />
</div>
