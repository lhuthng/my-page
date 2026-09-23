<script>
	import AudiobookPlayer from '$lib/components/audio/AudiobookPlayer.svelte';
	import BackButton from '$lib/components/ui/BackButton.svelte';
	import { absoluteSiteUrl, SITE_NAME, safeJsonLd } from '$lib/config/site.js';
	import { formatDurationLabel } from '$lib/utils/duration.js';

	let { data } = $props();

	const audiobook = $derived(data.audiobook);
	const durationLabel = $derived(formatDurationLabel(audiobook.total_duration_seconds));

	// Structured data so search engines can surface the audiobook and its
	// chapter list rather than treating the page as a generic article.
	const jsonLd = $derived(
		safeJsonLd({
			'@context': 'https://schema.org',
			'@type': 'Audiobook',
			name: audiobook.title,
			description: audiobook.description || undefined,
			inLanguage: 'en',
			image: absoluteSiteUrl(audiobook.url ?? '/thinkcats.jpg'),
			url: absoluteSiteUrl(`/audiobooks/${audiobook.slug}`),
			publisher: { '@type': 'Organization', name: SITE_NAME },
			author: audiobook.owner_display_name
				? { '@type': 'Person', name: audiobook.owner_display_name }
				: undefined,
			translator: audiobook.translator
				? { '@type': 'Person', name: audiobook.translator }
				: undefined,
			keywords: audiobook.tags?.join(', ') || undefined,
			numTracks: audiobook.tracks.length,
			track: audiobook.tracks.map((track, index) => ({
				'@type': 'MusicRecording',
				name: track.title,
				position: index + 1,
				duration: track.duration_seconds ? `PT${track.duration_seconds}S` : undefined
			}))
		})
	);
</script>

<svelte:head>
	<title>{audiobook.title} | Huu Thang's Blog</title>
	<meta
		name="description"
		content={audiobook.description ||
			`${audiobook.title} — an audiobook with ${audiobook.tracks.length} chapters.`}
	/>
	<link rel="canonical" href={absoluteSiteUrl(`/audiobooks/${audiobook.slug}`)} />
	<meta property="og:title" content={audiobook.title} />
	<meta property="og:type" content="music.album" />
	<meta property="og:url" content={absoluteSiteUrl(`/audiobooks/${audiobook.slug}`)} />
	{#if audiobook.url}
		<meta property="og:image" content={absoluteSiteUrl(audiobook.url)} />
	{/if}
	<meta name="twitter:card" content="summary_large_image" />
	{#if audiobook.description}
		<meta property="og:description" content={audiobook.description} />
		<meta name="twitter:description" content={audiobook.description} />
	{/if}
	{@html `<script type="application/ld+json">${jsonLd}</script>`}
</svelte:head>

<article class="bg-white rounded-xl p-4 mb-2 md:mb-4 flex flex-col gap-4">
	<BackButton href="/audiobooks" text="Audiobooks" />

	<h1 class="text-3xl md:text-4xl font-bold">
		{audiobook.title}
	</h1>

	<div class="flex flex-col md:flex-row gap-4 items-start">
		{#if audiobook.url}
			<img
				src={audiobook.url}
				alt={`Cover of ${audiobook.title}`}
				class="w-full aspect-[1.91/1] md:w-40 md:h-40 rounded-xl object-cover shrink-0"
			/>
		{/if}

		<div class="flex flex-col gap-2 min-w-0">
			<div class="flex items-center gap-2 flex-wrap text-base text-dark/60">
				{#if audiobook.translator}
					<span>By {audiobook.translator}</span>
					<span aria-hidden="true">-</span>
				{/if}
				{#if audiobook.owner_display_name || audiobook.owner_username}
					<a href="/profiles/{audiobook.owner_username}" class="text-dark/70">
						{audiobook.owner_display_name || audiobook.owner_username}
					</a>
					<span aria-hidden="true">-</span>
				{/if}
				<span>{audiobook.tracks.length} chapter{audiobook.tracks.length === 1 ? '' : 's'}</span>
				{#if durationLabel}
					<span aria-hidden="true">-</span>
					<span>{durationLabel}</span>
				{/if}
			</div>

			{#if audiobook.description}
				<p class="text-base text-dark/80 whitespace-pre-line">{audiobook.description}</p>
			{/if}

			{#if audiobook.tags?.length}
				<ul class="flex flex-wrap gap-1">
					{#each audiobook.tags as tag (tag.id)}
						<li>
							<a
								href="/audiobooks?tag={tag.slug}"
								class="text-base bg-primary/20 px-2 py-0.5 rounded-full no-underline! text-dark hover:bg-primary/40"
							>
								{tag.name}
							</a>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	</div>

	<AudiobookPlayer
		tracks={audiobook.tracks}
		title={audiobook.title}
		author={audiobook.owner_display_name || audiobook.owner_username}
		translator={audiobook.translator ?? ''}
		coverUrl={audiobook.url}
		storageKey={audiobook.slug}
	/>
</article>
