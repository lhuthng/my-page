<script>
	import CommentButton from '../shell/buttons/CommentButton.svelte';

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
		stats,
		routePrefix = '/posts',
		dashboardPrefix = '/dashboard/posts/id',
		children,
		coverMediaType = ''
	} = $props();
	let expanded = $state(false);

	let link = $derived(dashboardMode ? `${dashboardPrefix}/${id}` : `${routePrefix}/${slug}`);
</script>

<div class="bg-white rounded-lg drop-shadow-sm h-full">
	<div
		class="relative flex flex-col gap-2 bg-background/40 hover:bg-background/60 transition-colors duration-50 rounded-lg h-full"
	>
		{#if !dashboardMode}
			<a
				class="relative block z-10 w-full aspect-[1.91/1] cursor-pointer rounded-t-lg origin-center hover:scale-102 transition-[scale,border-radius] duration-100 overflow-hidden hover:rounded-b-lg"
				href={status === 'draft' ? `${dashboardPrefix}/${id}` : link}
			>
				{#if coverMediaType?.startsWith('video/')}
					<video
						class="absolute z-10 left-0 top-0 w-full h-full object-cover bg-white border-3 border-dark rounded-t-lg"
						src={src ?? '/missing.png'}
						poster={src ?? '/missing.png'}
						muted
						loop
						playsinline
						autoplay
						preload="auto"
					></video>
				{:else}
					<img
						class="absolute z-10 left-0 top-0 w-full h-full object-cover bg-white border-3 border-dark rounded-t-lg"
						src={src ?? '/missing.png'}
						alt="post-cover"
					/>
				{/if}
				<div
					class="absolute flex items-center justify-center gap-4 z-11 left-0 right-0 bottom-0 h-8 bg-dark/80 text-sm font-semibold"
				>
					<div>
						<span class="text-white/80!">{stats?.views ?? '#'}</span>
						<svg class="inline-block fill-white/80 h-6" viewBox="0 0 24 24">
							<path
								d="M11.5 6C10.9477 6 10.5 6.44772 10.5 7C10.5 7.55228 10.9477 8 11.5 8H20C20.5523 8 21 7.55228 21 7C21 6.44772 20.5523 6 20 6H11.5ZM15 11C14.4477 11 14 11.4477 14 12C14 12.5523 14.4477 13 15 13H20C20.5523 13 21 12.5523 21 12C21 11.4477 20.5523 11 20 11H15ZM12 16C11.4477 16 11 16.4477 11 17C11 17.5523 11.4477 18 12 18H20C20.5523 18 21 17.5523 21 17C21 16.4477 20.5523 16 20 16H12ZM7.70711 8.29289C7.31658 7.90237 6.68342 7.90237 6.29289 8.29289C5.90237 8.68342 5.90237 9.31658 6.29289 9.70711L7.58579 11H4C3.44772 11 3 11.4477 3 12C3 12.5523 3.44772 13 4 13H7.58579L6.29289 14.2929C5.90237 14.6834 5.90237 15.3166 6.29289 15.7071C6.68342 16.0976 7.31658 16.0976 7.70711 15.7071L10.7071 12.7071C11.0976 12.3166 11.0976 11.6834 10.7071 11.2929L7.70711 8.29289Z"
							></path>
						</svg>
					</div>
					<div>
						<svg class="inline-block fill-white/80 h-6" viewBox="0 0 26 26">
							<path
								fill-rule="evenodd"
								clip-rule="evenodd"
								d="M12.7071 4.29289C12.5196 4.10536 12.2652 4 12 4C11.7348 4 11.4804 4.10536 11.2929 4.29289L4.29289 11.2929C3.90237 11.6834 3.90237 12.3166 4.29289 12.7071C4.68342 13.0976 5.31658 13.0976 5.70711 12.7071L12 6.41421L18.2929 12.7071C18.6834 13.0976 19.3166 13.0976 19.7071 12.7071C20.0976 12.3166 20.0976 11.6834 19.7071 11.2929L12.7071 4.29289ZM12.7071 11.2929C12.5196 11.1054 12.2652 11 12 11C11.7348 11 11.4804 11.1054 11.2929 11.2929L4.29289 18.2929C3.90237 18.6834 3.90237 19.3166 4.29289 19.7071C4.68342 20.0976 5.31658 20.0976 5.70711 19.7071L12 13.4142L18.2929 19.7071C18.6834 20.0976 19.3166 20.0976 19.7071 19.7071C20.0976 19.3166 20.0976 18.6834 19.7071 18.2929L12.7071 11.2929Z"
							></path>
						</svg>
						<span class="text-white/80!">{stats?.likes ?? '#'}</span>
					</div>
				</div>
			</a>
		{:else}
			<button class="relative block z-10 w-full aspect-[1.91/1]" {onclick}>
				{#if coverMediaType?.startsWith('video/')}
					<video
						class="absolute z-10 left-0 top-0 w-full h-full object-cover rounded-t-lg origin-center transition-[scale,border-radius] duration-100 cursor-pointer hover:scale-102 bg-white border-3 border-dark hover:rounded-b-lg"
						src={src ?? '/missing.png'}
						poster={src ?? '/missing.png'}
						muted
						loop
						playsinline
						autoplay
						preload="auto"
					></video>
				{:else}
					<img
						class="absolute z-10 left-0 top-0 w-full h-full object-cover rounded-t-lg origin-center transition-[scale,border-radius] duration-100 cursor-pointer hover:scale-102 bg-white border-3 border-dark hover:rounded-b-lg"
						src={src ?? '/missing.png'}
						alt="post-cover"
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
				<h1 class="text-md md:text-lg line-clamp-2">
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
			{#if excerpt}
				<div
					class="grid transition-[grid-template-rows] duration-300 ease-out"
					class:grid-rows-[1fr]={expanded}
					class:grid-rows-[0fr]={!expanded}
				>
					<div class="overflow-hidden min-h-0">
						<div
							class="transition-opacity duration-200"
							class:opacity-100={expanded}
							class:opacity-0={!expanded}
						>
							<p>{excerpt}</p>
						</div>
					</div>
				</div>
				<svg
					class="expand-btn h-6 w-12 transition-transform duration-200 block mx-auto fill-primary/20 has-hover:fill-dark/60 z-9"
					class:toggled={expanded}
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

<style lang="postcss">
	@reference "../../../app.css";

	span {
		@apply text-dark/50;
	}

	.tag-container {
		@apply pointer-events-none;
		& > li {
			@apply h-4;
		}
	}

	.expand-btn.toggled {
		@apply -rotate-180;
	}
</style>
