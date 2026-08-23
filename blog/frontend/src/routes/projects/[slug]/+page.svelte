<script>
	import { page } from '$app/state';
	import { browser } from '$app/environment';
	import { dateTillNow, textToDate } from '$lib/utils';
	import { isLiked, shouldSendView, VIEW_DELAY } from '$lib/utils/post';
	import PostSection from '$lib/components/post/PostSection.svelte';
	import CommentSection from '$lib/components/post/CommentSection.svelte';
	import ProjectDemo from '$lib/components/project/ProjectDemo.svelte';
	import BackButton from '$lib/components/ui/BackButton.svelte';
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
		demo_type,
		delegate_game_id,
		delegated_game,
		initialVariant,
		links
	} = $derived(data);

	// A delegated project plays its game's launcher.
	let effectiveDemo = $derived(
		demo_type === 'game' && delegated_game
			? {
					type: delegated_game.launcher_type,
					url: delegated_game.demo_url,
					width: delegated_game.demo_width,
					height: delegated_game.demo_height,
					v86Runtime: delegated_game.v86_runtime
				}
			: {
					type: demo_type,
					url: demo_url,
					width: demo_width,
					height: demo_height,
					v86Runtime: undefined
				}
	);

	let isDelegationOrphan = $derived(
		demo_type === 'game' && !delegated_game && delegate_game_id != null
	);

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
				'@type': 'CreativeWork',
				'@id': `${canonicalLink}#project`,
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
						name: 'Projects',
						item: `${SITE_ORIGIN}/projects`
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
	{#if isDelegationOrphan}
		<section class="rounded-xl border border-amber-300 bg-amber-50 p-4 text-amber-900">
			<h2 class="font-semibold">Game unavailable</h2>
			<p class="mt-1 text-sm leading-relaxed">
				This project’s playable demo pointed at a game that was removed or is no longer published.
				The game’s files and saves were moved to trash and will be permanently deleted after 7 days.
				The article remains. The author can restore the game or point this project at another game.
			</p>
			<a href="/games" class="mt-2 inline-block text-sm underline">Browse games</a>
		</section>
	{:else if effectiveDemo.type !== 'none'}
		<ProjectDemo
			{title}
			demoType={effectiveDemo.type}
			demoUrl={effectiveDemo.url}
			v86Runtime={effectiveDemo.v86Runtime}
			{initialVariant}
			width={effectiveDemo.width ?? '100%'}
			height={effectiveDemo.height ?? '520px'}
		/>
	{/if}

	{#if links?.length > 0}
		<section class="flex flex-col gap-2 bg-white rounded-xl p-4">
			<div class="space-y-2">
				{#if demo_type === 'none'}
					<BackButton href="/projects" text="All projects" />
				{/if}
				<div class="flex items-center gap-3 mb-3">
					<h2 class="text-xl lg:text-2xl">Sources</h2>
					<hr class="grow border" />
				</div>
			</div>
			<ul class="flex flex-wrap gap-2">
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
		hideBackButton={demo_type !== 'none' || links?.length > 0}
	/>

	<CommentSection postId={post_id} postAuthorUsername={author_slug} />
</article>
