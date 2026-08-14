<script>
	import CommentButton from '../shell/buttons/CommentButton.svelte';
	import GridExpander from '../shell/GridExpander.svelte';
	import { lazyVideo } from '$lib/actions/lazyVideo.js';

	let {
		src,
		id,
		status,
		dashboardMode,
		onclick,
		title,
		slug,
		series,
		excerpt,
		author,
		tags,
		previewMode,
		routePrefix = '/posts',
		dashboardPrefix = '/dashboard/posts/id',
		children,
		coverMediaType = '',
		readingTime = 0
	} = $props();
	let expanded = $state(false);

	let link = $derived(dashboardMode ? `${dashboardPrefix}/${id}` : `${routePrefix}/${slug}`);
	let coverSrc = $derived(src ?? '/missing.png');

	let readingTier = $derived(
		readingTime <= 0
			? null
			: readingTime < 4
				? 'green'
				: readingTime < 8
					? 'blue'
					: readingTime < 16
						? 'yellow'
						: readingTime < 32
							? 'orange'
							: 'red'
	);
</script>

<div class="bg-white rounded-lg drop-shadow-sm h-full">
	<div
		class="relative flex flex-col gap-2 bg-background/40 hover:bg-background/60 transition-colors duration-50 rounded-lg h-full"
	>
		{#if !dashboardMode}
			<a
				class="reading-cover relative block z-10 w-full aspect-[1.91/1] cursor-pointer rounded-t-lg origin-center hover:scale-102 transition-[scale,border-radius] duration-100 overflow-hidden hover:rounded-b-lg hover:[&>.reading-bar]:rounded-b-lg"
				href={status === 'draft' ? `${dashboardPrefix}/${id}` : link}
				data-awareness={readingTier ?? 'none'}
			>
				{#if coverMediaType?.startsWith('video/')}
					<video
						class="reading-media absolute z-10 left-0 top-0 w-full h-full object-cover bg-white border-3 rounded-t-lg"
						data-src={coverSrc}
						poster={`${coverSrc}.thumbnail`}
						muted
						loop
						playsinline
						preload="none"
						use:lazyVideo
					></video>
				{:else}
					<img
						class="reading-media absolute z-10 left-0 top-0 w-full h-full object-cover bg-white border-3 rounded-t-lg"
						src={coverSrc}
						alt="post-cover"
						loading="lazy"
						decoding="async"
					/>
				{/if}
				<div
					class="reading-bar absolute flex items-center justify-center z-11 left-0 right-0 bottom-0 h-8 text-sm font-semibold transition-[scale,border-radius] duration-100 border-3"
				>
					{#if readingTime > 0}
						<span class="flex items-center gap-1" title={`${readingTime} min read`}>
							<svg class="inline-block h-6 w-6" viewBox="0 0 24 24">
								<path
									d="M5.06152 12C5.55362 8.05369 8.92001 5 12.9996 5C17.4179 5 20.9996 8.58172 20.9996 13C20.9996 17.4183 17.4179 21 12.9996 21H8M13 13V9M11 3H15M3 15H8M5 18H10"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
								></path>
							</svg>
{readingTime}m
						</span>
					{:else}
						<span>...</span>
					{/if}
				</div>
			</a>
		{:else}
			<button class="relative block z-10 w-full aspect-[1.91/1]" {onclick}>
				{#if coverMediaType?.startsWith('video/')}
					<video
						class="absolute z-10 left-0 top-0 w-full h-full object-cover rounded-t-lg origin-center transition-[scale,border-radius] duration-100 cursor-pointer hover:scale-102 bg-white border-3 border-dark hover:rounded-b-lg"
						data-src={coverSrc}
						poster={`${coverSrc}.thumbnail`}
						muted
						loop
						playsinline
						preload="none"
						use:lazyVideo
					></video>
				{:else}
					<img
						class="absolute z-10 left-0 top-0 w-full h-full object-cover rounded-t-lg origin-center transition-[scale,border-radius] duration-100 cursor-pointer hover:scale-102 bg-white border-3 border-dark hover:rounded-b-lg"
						src={coverSrc}
						alt="post-cover"
						loading="lazy"
						decoding="async"
					/>
				{/if}
				{#if children !== undefined}
					{@render children()}
				{/if}
			</button>
		{/if}
		<div class="relative z-10 w-full px-3 pb-2 min-w-0">
			<a
				class="w-fit"
				href={status === 'draft' ? `${dashboardPrefix}/${id}` : !dashboardMode ? link : undefined}
			>
				<h1 class="text-md md:text-lg line-clamp-2 leading-6">
					{title}
					{#if status === 'draft'}
						<i class="text-accent-red">(draft)</i>
					{:else if dashboardMode}
						<i class="text-accent-red">(dashboard)</i>
					{/if}
				</h1>
			</a>
			<div class="flex text-sm sm:text-md pr-4">
				<span class="select-none pointer-events-auto">
					by <a
						class="select-text text-dark!"
						href={!dashboardMode ? `/profiles/${author.slug}` : undefined}
					>
						{author.name}
					</a>
				</span>
				{#if series !== undefined}
					<div class="flex grow shrink gap-2 text-dark/50">
						<span>;</span>
						<span class="pointer-events-auto">
							<span>::{series.order}</span>
							from
							<a class="text-dark!" href={`/series/${series.slug}`}>{series.name}</a>
						</span>
					</div>
				{/if}
			</div>
			<div class="flex text-sm sm:text-md gap-1 grow shrink mb-2">
				{#if tags?.length > 0}
					<span class="text-dark/50">tags:</span>
				{/if}
				<ul
					class="pointer-events-none [&>li]:h-4 flex flex-wrap h-fit gap-y-2 sm:gap-y-0.5 gap-x-1 pr-2"
				>
					{#each tags as tag}
						<li>
							<a href={`/tags/${tag.replace(' ', '-')}`}>#{tag}</a>
						</li>
					{/each}
				</ul>
			</div>
			{#if excerpt}
				<GridExpander {expanded} duration="300ms">
					<div
						class="transition-opacity duration-200"
						class:opacity-100={expanded}
						class:opacity-0={!expanded}
					>
						<p>{excerpt}</p>
					</div>
				</GridExpander>
				<svg
					class="expand-btn h-6 w-12 transition-transform duration-200 block mx-auto fill-primary/20 has-hover:fill-dark/60 z-9"
					class:-rotate-180={expanded}
					class:translate-y-2={expanded}
					xmlns="http://www.w3.org/2000/svg"
					viewBox="0 0 32 32"
				>
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<polygon
						class="pointer-events-auto cursor-pointer focus:outline-none"
						points="0,16 32,16 16,32"
						role="button"
						tabindex="0"
						onclick={() => (expanded = !expanded)}
					/>
				</svg>
			{/if}
		</div>
	</div>
</div>
