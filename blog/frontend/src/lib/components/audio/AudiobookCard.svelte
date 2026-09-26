<script>
	import GridExpander from '../shell/GridExpander.svelte';
	import Book from '../svgs/Book.svelte';
	import VietnameseFlagBadge from './VietnameseFlagBadge.svelte';
	import { isVietnameseTranslation } from '$lib/utils/audiobook-tags.js';

	let { audiobook } = $props();

	const {
		title,
		slug,
		description,
		translator,
		url,
		track_count,
		tags = [],
		tag_slugs = [],
		status
	} = $derived(audiobook);

	let expanded = $state(false);

	const link = $derived(`/audiobooks/${slug}`);
	const coverSrc = $derived(url ?? '/missing.png');
	const vietnameseTranslated = $derived(isVietnameseTranslation(audiobook));
</script>

<div class="bg-white rounded-lg drop-shadow-sm h-full">
	<div
		class="relative flex flex-col gap-2 bg-background/40 hover:bg-background/60 transition-colors duration-50 rounded-lg h-full"
	>
		<a
			class="reading-cover relative block z-10 w-full aspect-[1.91/1] cursor-pointer rounded-t-lg origin-center hover:scale-102 transition-[scale,border-radius] duration-100 overflow-hidden hover:rounded-b-lg hover:[&>.reading-bar]:rounded-b-lg"
			href={link}
		>
			<img
				class="reading-media absolute z-10 left-0 top-0 w-full h-full object-cover bg-white border-3 rounded-t-lg"
				src={coverSrc}
				alt="audiobook-cover"
				loading="lazy"
				decoding="async"
			/>
			<div
				class="reading-bar absolute flex items-center justify-center z-11 left-0 right-0 bottom-0 h-8 text-sm font-semibold transition-[scale,border-radius] duration-100 border-3"
			>
				{#if track_count != null}
					<span
						class="flex items-center gap-1"
						title={`${track_count} chapter${track_count === 1 ? '' : 's'}`}
					>
						<Book class="inline-block h-6 w-6" />
						{track_count}
						{track_count === 1 ? 'chapter' : 'chapters'}
					</span>
				{:else}
					<span>...</span>
				{/if}
			</div>
			{#if vietnameseTranslated}
				<VietnameseFlagBadge class="right-1.5 bottom-1.5" />
			{/if}
		</a>
		<div class="relative z-10 w-full px-3 pb-2 min-w-0">
			<a class="w-fit" href={link}>
				<h1 class="text-md md:text-lg line-clamp-2 leading-6">
					{title}
					{#if status && status !== 'published'}
						<i class="text-accent-red">({status})</i>
					{/if}
				</h1>
			</a>
			{#if translator}
				<div class="flex pr-4 text-sm sm:text-md">
					<span class="select-none">By {translator}</span>
				</div>
			{/if}
			<div class="flex text-sm sm:text-md gap-1 grow shrink mb-2">
				{#if tags?.length > 0}
					<span class="text-dark/50">tags:</span>
				{/if}
				<ul
					class="pointer-events-none [&>li]:h-4 flex flex-wrap h-fit gap-y-2 sm:gap-y-0.5 gap-x-1 pr-2"
				>
					{#each tags as tag, index}
						<li>
							<a href={`/audiobooks?tag=${tag_slugs[index] ?? tag}`}>#{tag}</a>
						</li>
					{/each}
				</ul>
			</div>
			{#if description}
				<GridExpander {expanded} duration="300ms">
					<div
						class="transition-opacity duration-200"
						class:opacity-100={expanded}
						class:opacity-0={!expanded}
					>
						<p>{description}</p>
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
