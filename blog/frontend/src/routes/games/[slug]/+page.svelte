<script>
	import { page } from '$app/state';
	import { browser } from '$app/environment';
	import { dateTillNow, textToDate } from '$lib/utils';
	import { isLiked, shouldSendView, VIEW_DELAY } from '$lib/utils/post';
	import PostSection from '$lib/components/post/PostSection.svelte';
	import CommentSection from '$lib/components/post/CommentSection.svelte';
	import GameDemo from '$lib/components/game/GameDemo.svelte';
	import {
		absoluteSiteUrl,
		canonicalUrl,
		safeJsonLd,
		SITE_AUTHOR,
		SITE_LOCALE,
		SITE_NAME,
		SITE_OG_IMAGE_HEIGHT,
		SITE_OG_IMAGE_WIDTH,
		SITE_ORIGIN
	} from '$lib/config/site.js';

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
		og_image_url,
		cover_video_url,
		cover_video_type,
		og_image_seconds,
		demo_url,
		demo_width,
		demo_height,
		launcher_type,
		v86_runtime,
		instruction,
		cheatcode,
		story,
		related_games,
		initialVariant
	} = $derived(data);

	let liked = $state();
	$effect(() => {
		liked = browser ? isLiked(post_id) : false;
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
				'@type': 'VideoGame',
				'@id': `${canonicalLink}#game`,
				mainEntityOfPage: canonicalLink,
				url: canonicalLink,
				name: title,
				headline: title,
				description: excerpt,
				image: ogImageUrl,
				datePublished: published_at,
				dateModified: updated_at ?? published_at,
				keywords: tags,
				inLanguage: 'en',
				author: {
					'@type': 'Person',
					name: author_name,
					url: canonicalUrl(`/profiles/${author_slug}`),
					sameAs: SITE_AUTHOR.sameAs
				},
				publisher: {
					'@type': 'Organization',
					'@id': `${SITE_ORIGIN}/#organization`,
					name: SITE_NAME,
					url: `${SITE_ORIGIN}/`,
					logo: { '@type': 'ImageObject', url: SITE_AUTHOR.image },
					publisher: { '@id': `${SITE_ORIGIN}/#person` }
				},
				isPartOf: {
					'@type': 'WebSite',
					'@id': `${SITE_ORIGIN}/#website`,
					name: SITE_NAME
				}
			},
			{
				'@type': 'Person',
				'@id': `${SITE_ORIGIN}/#person`,
				name: SITE_AUTHOR.name,
				alternateName: SITE_AUTHOR.alternateName,
				url: SITE_AUTHOR.url,
				image: SITE_AUTHOR.image,
				sameAs: SITE_AUTHOR.sameAs
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
						name: 'Games',
						item: `${SITE_ORIGIN}/games`
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
	<meta property="og:type" content="article" />
	<meta property="og:description" content={excerpt} />
	<meta property="og:image" content={ogImageUrl} />
	<meta property="og:image:width" content={SITE_OG_IMAGE_WIDTH} />
	<meta property="og:image:height" content={SITE_OG_IMAGE_HEIGHT} />
	<meta property="og:locale" content={SITE_LOCALE} />
	<meta property="article:published_time" content={published_at} />
	<meta property="article:modified_time" content={updated_at ?? published_at} />

	{#each tags as tag}
		<meta property="article:tag" content={tag} />
	{/each}

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
	<GameDemo
		{title}
		launcherType={launcher_type}
		demoUrl={demo_url}
		v86Runtime={v86_runtime}
		{initialVariant}
		width={demo_width ?? '100%'}
		height={demo_height ?? '520px'}
		{instruction}
		{cheatcode}
		{story}
	/>

	{#if related_games?.length > 0}
		<section class="flex flex-col gap-2 bg-white rounded-xl p-4">
			<div class="space-y-2">
				<div class="flex items-center gap-3 mb-3">
					<h2 class="text-xl lg:text-2xl">Related games</h2>
					<hr class="grow border" />
				</div>
			</div>
			<ul class="flex flex-wrap gap-2">
				{#each related_games as game (game.id)}
					<li class="duo-btn" data-duo-color="blue">
						<a class="no-underline!" href="/games/{game.slug}">{game.title}</a>
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
		editHref={`/dashboard/games/id/${id}`}
		author={{
			username: author_slug,
			displayName: author_name,
			avatarUrl: author_avatar_url
		}}
		hideBackButton={related_games?.length > 0}
	/>

	<CommentSection postId={post_id} postAuthorUsername={author_slug} />
</article>
