<script>
	import App from '$lib/components/embeds/App.svelte';
	import Introduction from '$lib/components/home/Introduction.svelte';
	import PostDiscovery from '$lib/components/home/PostDiscovery.svelte';
	import ProjectDiscovery from '$lib/components/home/ProjectDiscovery.svelte';
	import Suggestion from '$lib/components/home/Suggestion.svelte';
	import {
		absoluteSiteUrl,
		safeJsonLd,
		SITE_AUTHOR,
		SITE_DESCRIPTION,
		SITE_NAME,
		SITE_ORIGIN
	} from '$lib/config/site.js';

	const { data } = $props();

	const featuredPosts = $derived(data.featured_posts || []);
	const featuredProjects = $derived(data.featured_projects || []);

	const imageUrl = absoluteSiteUrl('/thinkcats.jpg');
	const structuredData = {
		'@context': 'https://schema.org',
		'@graph': [
			{
				'@type': 'WebSite',
				'@id': `${SITE_ORIGIN}/#website`,
				url: `${SITE_ORIGIN}/`,
				name: SITE_NAME,
				alternateName: ["Thắng's Blog", "Huu Thang's Blog"],
				description: SITE_DESCRIPTION,
				inLanguage: 'en',
				publisher: { '@id': `${SITE_ORIGIN}/#organization` }
			},
			{
				'@type': 'Organization',
				'@id': `${SITE_ORIGIN}/#organization`,
				name: SITE_NAME,
				url: `${SITE_ORIGIN}/`,
				logo: { '@type': 'ImageObject', url: imageUrl },
				publisher: { '@id': `${SITE_ORIGIN}/#person` }
			},
			{
				'@type': 'Person',
				'@id': `${SITE_ORIGIN}/#person`,
				name: SITE_AUTHOR.name,
				alternateName: SITE_AUTHOR.alternateName,
				url: SITE_AUTHOR.url,
				image: SITE_AUTHOR.image,
				sameAs: SITE_AUTHOR.sameAs,
				knowsAbout: [
					'software architecture',
					'systems design',
					'creative coding',
					'3D animation',
					'game development'
				]
			}
		]
	};
</script>

<svelte:head>
	<title>Home Sweet Home | Huu Thang's Blog</title>
	<meta
		name="description"
		content="Welcome to the Field! I'm Thang, and this is my digital garden where I document work experiences, hobbies, and experiments."
	/>

	<meta property="og:type" content="website" />
	<meta property="og:title" content="Home Sweet Home | Huu Thang's Blog" />
	<meta
		property="og:description"
		content="Deep dives into technical challenges and small creative sparks."
	/>
	<meta property="og:image" content={imageUrl} />

	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:image" content={imageUrl} />
	<meta
		name="twitter:description"
		content="Deep dives into technical challenges and small creative sparks."
	/>
	{@html `<script type="application/ld+json">${safeJsonLd(structuredData)}</script>`}
</svelte:head>

<div class="relative flex gap-4 z-5 *:h-fit pb-2 lg:pb-4">
	<div class="grow space-y-2 lg:space-y-4">
		{#if featuredProjects.length > 0}
			<div class="bg-white rounded-xl px-4 pb-4 overflow-hidden">
				<ProjectDiscovery {featuredProjects} />
			</div>
		{/if}
		<div class="bg-white rounded-xl px-4 pb-2 overflow-hidden">
			<PostDiscovery {featuredPosts} />
		</div>
	</div>
	<div class="not-lg:hidden w-60 bg-white rounded-xl">
		<Suggestion />
	</div>
</div>
<div class="pb-4">
	<Introduction />
</div>
