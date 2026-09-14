<script>
	import { formatDurationLabel } from '$lib/utils/duration.js';

	let { audiobook } = $props();

	const {
		title,
		slug,
		description,
		translator,
		url,
		track_count,
		total_duration_seconds,
		tags = [],
		tag_slugs = [],
		status,
		owner_display_name,
		owner_username
	} = $derived(audiobook);

	const href = $derived(`/audiobooks/${slug}`);
	const durationLabel = $derived(formatDurationLabel(total_duration_seconds));
</script>

<li class="flex not-lg:flex-col gap-4 bg-white rounded-xl p-4 hover:shadow-lg transition-shadow">
	<a {href} class="shrink-0">
		{#if url}
			<img
				src={url}
				alt={`Cover of ${title}`}
				class="w-full not-lg:max-w-40 h-40 lg:w-40 rounded-xl object-cover"
				loading="lazy"
			/>
		{:else}
			<div
				class="w-full not-lg:max-w-40 h-40 lg:w-40 rounded-xl bg-primary/20 flex items-center justify-center"
			>
				<svg class="w-12 h-12 fill-primary" viewBox="0 0 24 24" aria-hidden="true">
					<path
						d="M12 3a4 4 0 0 0-4 4v5a4 4 0 0 0 8 0V7a4 4 0 0 0-4-4zm-6 9a6 6 0 0 0 12 0h2a8 8 0 0 1-7 7.93V22h-2v-2.07A8 8 0 0 1 4 12z"
					/>
				</svg>
			</div>
		{/if}
	</a>

	<div class="flex flex-col gap-2 min-w-0 grow">
		<div class="flex items-start gap-2 flex-wrap">
			<h2 class="text-lg font-semibold grow">
				<a {href} class="no-underline! text-dark hover:underline">{title}</a>
			</h2>
			{#if status !== 'published'}
				<span class="text-xs bg-dark/10 px-2 py-0.5 rounded-full text-dark/60">{status}</span>
			{/if}
		</div>

		{#if translator}
			<p class="text-sm text-dark/60">Translated by {translator}</p>
		{:else if owner_display_name || owner_username}
			<p class="text-sm text-dark/60">{owner_display_name || owner_username}</p>
		{/if}

		{#if description}
			<p class="text-sm text-dark/70 line-clamp-3">{description}</p>
		{/if}

		<div class="flex items-center gap-2 flex-wrap text-xs text-dark/60">
			<span>{track_count} chapter{track_count === 1 ? '' : 's'}</span>
			{#if durationLabel}
				<span aria-hidden="true">·</span>
				<span>{durationLabel}</span>
			{/if}
		</div>

		{#if tags.length}
			<ul class="flex flex-wrap gap-1">
				{#each tags as tag, index (tag_slugs[index] ?? tag)}
					<li>
						<a
							href="/audiobooks?tag={tag_slugs[index]}"
							class="text-xs bg-primary/20 px-2 py-0.5 rounded-full no-underline! text-dark hover:bg-primary/40"
						>
							{tag}
						</a>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</li>
