<script>
	import { page } from '$app/state';
	import { browser } from '$app/environment';
	import { dateTillNow, textToDate } from '$lib/utils';
	import { isLiked, shouldSendView, VIEW_DELAY } from '$lib/utils/post';
	import PostSection from '$lib/components/post/PostSection.svelte';
	import CommentSection from '$lib/components/post/CommentSection.svelte';
	import ProjectDemo from '$lib/components/project/ProjectDemo.svelte';

	let { data } = $props();

	let {
		id,
		post_id,
		author_slug,
		author_name,
		author_avatar_url,
		title,
		content,
		excerpt,
		published_at,
		updated_at,
		tags,
		cover_url,
		demo_url,
		demo_width,
		demo_height,
		demo_type,
		links
	} = $derived(data);

	let liked = $state();
	$effect(() => {
		liked = browser ? isLiked(post_id) : false;
	});

	let date = $derived(textToDate(published_at));
	let updateTime = $derived(dateTillNow(updated_at, 'round'));
	let imageUrl = $derived(page.url.origin + cover_url);
	let canonicalLink = $derived(page.url.origin + page.url.pathname);

	let viewDelayTimeout = null;
	$effect(() => {
		if (viewDelayTimeout) clearTimeout(viewDelayTimeout);
		viewDelayTimeout = setTimeout(async () => {
			if (shouldSendView(post_id)) {
				await fetch(`/api/posts/id/${post_id}/view`, { method: 'POST' });
			}
		}, VIEW_DELAY);
		return () => {
			if (viewDelayTimeout) {
				clearTimeout(viewDelayTimeout);
				viewDelayTimeout = null;
			}
		};
	});
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={excerpt} />

	<meta property="og:title" content={title} />
	<meta property="og:type" content="website" />
	<meta property="og:url" content={canonicalLink} />
	<meta property="og:description" content={excerpt} />
	<meta property="og:image" content={imageUrl} />

	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:url" content={canonicalLink} />
	<meta name="twitter:title" content={title} />
	<meta name="twitter:description" content={excerpt} />
	<meta name="twitter:image" content={imageUrl} />

	<link rel="canonical" href={canonicalLink} />
</svelte:head>

<article class="flex flex-col gap-4 pb-4 *:drop-shadow-xl">
	<ProjectDemo
		{title}
		demoType={demo_type}
		demoUrl={demo_url}
		width={demo_width ?? '100%'}
		height={demo_height ?? '520px'}
	/>

	{#if links?.length > 0}
		<section class="flex flex-col gap-2 bg-white rounded-xl p-4">
			<div class="flex items-center gap-3 mb-3">
				<h2 class="text-xl lg:text-2xl">Sources</h2>
				<hr class="grow border" />
			</div>
			<ul class="flex flex-wrap">
				{#each links as link}
					<li class="duo-btn" data-duo-color="blue">
						<a class="no-underline!" href={link.url} target="_blank" rel="noopener noreferrer">
							{link.label}
						</a>
					</li>
				{/each}
			</ul>
		</section>
	{/if}

	<PostSection
		id={post_id}
		{title}
		{tags}
		{date}
		{updateTime}
		{content}
		{liked}
		editHref={`/dashboard/projects/id/${id}`}
		author={{
			username: author_slug,
			displayName: author_name,
			avatarUrl: author_avatar_url
		}}
	/>

	<CommentSection postId={post_id} postAuthorUsername={author_slug} />
</article>
