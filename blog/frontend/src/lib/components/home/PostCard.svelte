<script>
	import CommentButton from '../shell/buttons/CommentButton.svelte';
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

	let toggled = $state(false);

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

<div class="bg-white rounded-lg drop-shadow-sm">
	<div
		class="relative flex gap-4 bg-background/40 hover:bg-background/60 transition-colors duration-50 rounded-lg"
	>
		{#if !dashboardMode}
			<a
				class="reading-cover relative block z-10 min-w-26 min-h-26 md:min-w-34 md:min-h-34 cursor-pointer rounded-lg origin-center hover:scale-105 transition-transform duration-100 overflow-hidden"
				href={status === 'draft' ? `${dashboardPrefix}/${id}` : link}
				data-awareness={readingTier ?? 'none'}
			>
				{#if coverMediaType?.startsWith('video/')}
					<video
						class="reading-media absolute z-10 left-0 top-0 w-26 h-26 md:w-34 md:h-34 object-cover bg-white border-3 rounded-lg"
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
						class="reading-media absolute z-10 left-0 top-0 w-26 h-26 md:w-34 md:h-34 object-cover bg-white border-3 rounded-lg"
						src={coverSrc}
						alt="post-cover"
						loading="lazy"
						decoding="async"
					/>
				{/if}
				<div
					class="reading-bar absolute flex items-center justify-center z-11 left-0 right-0 bottom-0 h-8 text-sm font-semibold rounded-b-lg border-3"
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
					{/if}
				</div>
			</a>
		{:else}
			<button class="relative block z-10 min-w-26 min-h-26 md:min-w-34 md:min-h-34" {onclick}>
				{#if coverMediaType?.startsWith('video/')}
					<video
						class="absolute z-10 left-0 top-0 w-26 h-26 md:w-34 md:h-34 object-cover rounded-lg origin-center transition-transform duration-100 cursor-pointer hover:scale-105 bg-white border-3 border-dark"
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
						class="absolute z-10 left-0 top-0 w-26 h-26 md:w-34 md:h-34 object-cover rounded-lg origin-center transition-transform duration-100 cursor-pointer hover:scale-105 bg-white border-3 border-dark"
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
		<div class="relative z-10 w-full pointer-events-none rounded-r-lg overflow-hidden">
			<div
				class="card absolute w-[200%] h-full flex transition-transform duration-200 left-0"
				class:toggled
			>
				<div class="h-full w-1/2 min-w-0">
					<div class="flex flex-col full py-2 min-w-0">
						<a
							class="w-fit pr-2"
							href={status === 'draft'
								? `${dashboardPrefix}/${id}`
								: !dashboardMode
									? link
									: undefined}
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
								<div class="flex grow shrink gap-2">
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
								<span>tags:</span>
							{/if}
							<ul class="tag-container flex flex-wrap h-fit gap-y-2 sm:gap-y-0.5 gap-x-1 pr-2">
								{#each tags as tag}
									<li>
										<a href={`/tags/${tag.replace(' ', '-')}`}>#{tag}</a>
									</li>
								{/each}
							</ul>
						</div>
					</div>
				</div>
				<div class="absolute left-1/2 h-full w-1/2" class:toggled>
					<div
						class="full p-2 pointer-events-auto text-sm sm:text-base overscroll-contain custom-scrollbar overflow-y-scroll"
					>
						<p>{excerpt}</p>
						<a
							class="block text-right"
							href={status === 'draft'
								? `${dashboardPrefix}/${id}`
								: !dashboardMode
									? link
									: undefined}
						>
							<span class="select-none">>{' '}</span>
							to page
						</a>
					</div>
				</div>
				<svg
					class="card-btn absolute top-0 left-1/2 h-full -translate-x-2/5 has-hover:-translate-x-1/2 has-hover:fill-primary/60 fill-primary/20 transition-all duration-200 z-9"
					class:toggled
					xmlns="http://www.w3.org/2000/svg"
					viewBox="0 0 32 32"
				>
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<polygon
						class="left pointer-events-auto cursor-pointer focus:outline-none"
						points="16,0 16,32 0,16"
						role="button"
						tabindex="0"
						onclick={() => (toggled = true)}
					/>
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<polygon
						class="right pointer-events-auto cursor-pointer focus:outline-none"
						points="16,0 16,32 32,16"
						role="button"
						tabindex="0"
						onclick={() => (toggled = false)}
					/>
				</svg>
			</div>
		</div>
	</div>
</div>

<style lang="postcss">
	@reference "../../../app.css";

	.tag-container {
		@apply pointer-events-none;
		& > li {
			@apply h-4;
		}
	}

	.card.toggled {
		@apply -translate-x-1/2;
	}

	.card-btn {
		& > polygon {
			@apply transition-opacity duration-300;
		}
		& > .left {
			@apply opacity-100;
		}
		& > .right {
			@apply opacity-0;
		}

		&.toggled {
			@apply -translate-x-3/5 has-hover:-translate-x-1/2;
			& > .left {
				@apply opacity-0;
			}
			& > .right {
				@apply opacity-100;
			}
		}
	}
</style>
