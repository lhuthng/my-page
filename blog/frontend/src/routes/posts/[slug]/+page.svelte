<script>
	import { dateTillNow, textToDate } from '$lib/utils';
	import PostSection from '$lib/components/post/PostSection.svelte';
	import CommentSection from '$lib/components/post/CommentSection.svelte';
	import { page } from '$app/state';
	import { isLiked, shouldSendView, VIEW_DELAY } from '$lib/utils/post';
	import { browser } from '$app/environment';
	import {
		absoluteSiteUrl,
		canonicalUrl,
		safeJsonLd,
		SITE_NAME,
		SITE_ORIGIN
	} from '$lib/config/site.js';

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
		og_image_url,
		cover_video_url,
		cover_video_type,
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
		return absoluteSiteUrl(cover_url, '/thinkcats.jpg');
	});

	let ogImageUrl = $derived.by(() => {
		return og_image_url ? absoluteSiteUrl(og_image_url) : imageUrl;
	});
	let canonicalLink = $derived(canonicalUrl(page.url.pathname));
	let structuredData = $derived({
		'@context': 'https://schema.org',
		'@graph': [
			{
				'@type': 'BlogPosting',
				'@id': `${canonicalLink}#article`,
				mainEntityOfPage: canonicalLink,
				url: canonicalLink,
				headline: title,
				description: excerpt,
				image: [ogImageUrl],
				datePublished: published_at,
				dateModified: updated_at ?? published_at,
				keywords: tags,
				inLanguage: 'en',
				author: {
					'@type': 'Person',
					name: author_name,
					url: canonicalUrl(`/profiles/${author_slug}`)
				},
				publisher: {
					'@type': 'Person',
					'@id': `${SITE_ORIGIN}/#person`,
					name: 'Huu Thang Le',
					url: `${SITE_ORIGIN}/about`
				},
				isPartOf: {
					'@type': 'WebSite',
					'@id': `${SITE_ORIGIN}/#website`,
					name: SITE_NAME
				}
			},
			{
				'@type': 'BreadcrumbList',
				itemListElement: [
					{
						'@type': 'ListItem',
						position: 1,
						name: 'Home',
						item: `${SITE_ORIGIN}/`
					},
					{
						'@type': 'ListItem',
						position: 2,
						name: 'Posts',
						item: `${SITE_ORIGIN}/posts`
					},
					{
						'@type': 'ListItem',
						position: 3,
						name: title,
						item: canonicalLink
					}
				]
			}
		]
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
	<meta property="og:description" content={excerpt} />
	<meta property="og:image" content={ogImageUrl} />
	<meta property="article:published_time" content={published_at} />
	<meta property="article:modified_time" content={updated_at ?? published_at} />

	{#if cover_video_url}
		<meta property="og:video" content={absoluteSiteUrl(cover_video_url)} />
		<meta property="og:video:type" content={cover_video_type} />
	{/if}

	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:url" content={canonicalLink} />
	<meta name="twitter:title" content={title} />
	<meta name="twitter:description" content={excerpt} />
	<meta name="twitter:image" content={ogImageUrl} />
	{@html `<script type="application/ld+json">${safeJsonLd(structuredData)}</script>`}
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
