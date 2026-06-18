<script>
	import { dateTillNow, textToDate } from '$lib/utils';
	import PostSection from '$lib/components/post/PostSection.svelte';
	import CommentSection from '$lib/components/post/CommentSection.svelte';
	import { page } from '$app/state';
	import { isLiked, shouldSendView, VIEW_DELAY } from '$lib/utils/post';
	import { browser } from '$app/environment';

	let { data, slug } = $props();

	let {
		id,
		author_slug,
		author_name,
		author_avatar_url,
		title,
		content,
		excerpt,
		published_at,
		updated_at,
		tags,
		series,
		cover_url,
		cover_media_type,
		og_image_seconds,
		relatedPosts
	} = $derived(data);
	let liked = $state();
	$effect(() => {
		liked = browser ? isLiked(id) : false;
	});
	let date = $derived(textToDate(published_at));
	let updateTime = $derived(dateTillNow(updated_at, 'round'));

	let imageUrl = $derived.by(() => {
		if (cover_url) {
			return page.url.origin + cover_url;
		}
		return page.url.origin + '/thinkcats.jpg';
	});

	let ogImageUrl = $derived.by(() => {
		if (cover_media_type?.startsWith('video/') && cover_url) {
			const parts = cover_url.split('/');
			const shortName = parts[parts.length - 1];
			return `${page.url.origin}/api/media/thumbnail?short_name=${shortName}`;
		}
		return imageUrl;
	});
	let canonicalLink = $derived.by(() => {
		const { origin, pathname, search } = page.url;
		return origin + pathname + search;
	});

	let viewDelayTimeout = null;
	$effect(() => {
		if (viewDelayTimeout) {
			clearTimeout(viewDelayTimeout);
		}

		viewDelayTimeout = setTimeout(async () => {
			if (shouldSendView(id)) {
				const res = await fetch(`/api/posts/id/${id}/view`, { method: 'POST' });
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
	<meta property="og:type" content="article" />
	<meta property="og:url" content={canonicalLink} />
	<meta property="og:description" content={excerpt} />
	<meta property="og:image" content={ogImageUrl} />

	{#if cover_media_type?.startsWith('video/') && cover_url}
		<meta property="og:video" content={page.url.origin + cover_url} />
		<meta property="og:video:type" content={cover_media_type} />
	{/if}

	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:url" content={canonicalLink} />
	<meta name="twitter:title" content={title} />
	<meta name="twitter:description" content={excerpt} />
	<meta name="twitter:image" content={ogImageUrl} />

	<link rel="canonical" href={canonicalLink} />
</svelte:head>

<article class="flex flex-col gap-4 pb-4 *:drop-shadow-xl">
	<PostSection
		{id}
		{title}
		{tags}
		{date}
		{updateTime}
		{content}
		{series}
		{liked}
		{relatedPosts}
		author={{
			username: author_slug,
			displayName: author_name,
			avatarUrl: author_avatar_url
		}}
	/>

	<CommentSection postId={id} postAuthorUsername={author_slug} />
</article>
