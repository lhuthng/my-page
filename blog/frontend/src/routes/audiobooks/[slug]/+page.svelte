<script>
	import AudiobookPlayer from '$lib/components/audio/AudiobookPlayer.svelte';
	import VietnameseFlagBadge from '$lib/components/audio/VietnameseFlagBadge.svelte';
	import BackButton from '$lib/components/ui/BackButton.svelte';
	import { absoluteSiteUrl, SITE_NAME, safeJsonLd } from '$lib/config/site.js';
	import { formatDurationLabel } from '$lib/utils/duration.js';
	import { isVietnameseTranslation } from '$lib/utils/audiobook-tags.js';

	let { data } = $props();

	const audiobook = $derived(data.audiobook);
	const vietnamese = $derived(isVietnameseTranslation(audiobook));
	const durationLabel = $derived(
		formatDurationLabel(audiobook.total_duration_seconds, vietnamese ? 'vi' : 'en')
	);

	// A Vietnamese-translated book is presented to Vietnamese readers, so the
	// detail labels render in Vietnamese; everything else stays in English.
	const t = $derived(
		vietnamese
			? {
					author: 'Tác giả:',
					uploadedBy: 'Đăng tải bởi',
					chapters: (n) => `${n} chương`,
					description: 'Mô tả'
				}
			: {
					author: 'Author:',
					uploadedBy: 'Uploaded by',
					chapters: (n) => `${n} chapter${n === 1 ? '' : 's'}`,
					description: 'Description'
				}
	);

	// Structured data so search engines can surface the audiobook and its
	// chapter list rather than treating the page as a generic article.
	const jsonLd = $derived(
		safeJsonLd({
			'@context': 'https://schema.org',
			'@type': 'Audiobook',
			name: audiobook.title,
			description: audiobook.description || undefined,
			inLanguage: vietnamese ? 'vi' : 'en',
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

<article class="flex flex-col gap-4 pb-4 *:drop-shadow-xl">
	<header class="flex flex-col gap-4 rounded-xl bg-white p-4">
		<BackButton href="/audiobooks" text="Audiobooks" />

		<h1 class="text-2xl break-words lg:text-4xl">
			{audiobook.title}
		</h1>

		<div class="flex flex-col items-start gap-4 md:flex-row">
			{#if audiobook.url}
				<div
					class="reading-cover relative w-full shrink-0 overflow-hidden rounded-xl border-3 border-dark bg-white md:w-64"
				>
					<img
						src={audiobook.url}
						alt={`Cover of ${audiobook.title}`}
						class="reading-media aspect-[1.91/1] w-full object-cover"
					/>
					{#if vietnamese}
						<VietnameseFlagBadge class="right-2 bottom-2" />
					{/if}
				</div>
			{/if}

			<div class="flex min-w-0 flex-1 flex-col gap-3">
				<div
					class="flex flex-wrap items-center gap-x-2 gap-y-1 text-base text-dark/60"
					lang={vietnamese ? 'vi' : undefined}
				>
					{#if audiobook.translator}
						<span>{t.author} {audiobook.translator}</span>
						<span class="text-dark/25" aria-hidden="true">•</span>
					{/if}
					{#if audiobook.owner_display_name || audiobook.owner_username}
						<span>{t.uploadedBy}</span>
						<a
							href="/profiles/{audiobook.owner_username}"
							class="text-accent-blue-dark hover:text-accent-blue"
						>
							{audiobook.owner_display_name || audiobook.owner_username}
						</a>
						<span class="text-dark/25" aria-hidden="true">•</span>
					{/if}
					<span>{t.chapters(audiobook.tracks.length)}</span>
					{#if durationLabel}
						<span class="text-dark/25" aria-hidden="true">•</span>
						<span>{durationLabel}</span>
					{/if}
				</div>

				{#if audiobook.tags?.length}
					<div class="inline gap-2 text-dark/60">
						<ul class="inline text-dark *:inline space-x-1" aria-label="Audiobook tags">
							{#each audiobook.tags as tag (tag.id)}
								<li
									class="rounded-full border-2 border-primary px-1 *:no-underline! has-hover:bg-primary duration-100 transition-colors"
								>
									<a
										class="inline-block text-primary duration-100 transition-colors hover:text-white hover:*:text-white"
										href="/audiobooks?tag={tag.slug}"
									>
										<span class="text-gray-300">#</span>
										{tag.name}
									</a>
								</li>
							{/each}
						</ul>
					</div>
				{/if}
			</div>
		</div>
	</header>

	{#if audiobook.description}
		<section class="rounded-xl bg-white p-4" lang={vietnamese ? 'vi' : undefined}>
			<div class="flex items-center gap-3 mb-3">
				<h2 class="text-xl lg:text-2xl">{t.description}</h2>
				<hr class="grow border" />
			</div>
			<p class="text-base leading-7 break-words whitespace-pre-line text-dark/75">
				{audiobook.description}
			</p>
		</section>
	{/if}

	<AudiobookPlayer
		tracks={audiobook.tracks}
		title={audiobook.title}
		author={audiobook.owner_display_name || audiobook.owner_username}
		translator={audiobook.translator ?? ''}
		coverUrl={audiobook.url}
		storageKey={audiobook.slug}
		slug={audiobook.slug}
		{vietnamese}
	/>
</article>
