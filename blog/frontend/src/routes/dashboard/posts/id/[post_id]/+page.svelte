<script>
	import PostEditor from '$lib/components/post/PostEditor.svelte';

	const { data } = $props();

	// Reactive, not a one-time `untrack` snapshot — see the equivalent comment
	// in the project dashboard edit page for why.
	const mapped = $derived({
		id: data.id,
		title: data.title,
		slug: data.slug,
		coverUrl: data.cover_url,
		cover_media_type: data.cover_media_type,
		videoShortName: data.video_short_name,
		ogImageSeconds: data.og_image_seconds,
		seriesSlug: data.series_slug,
		content: data.content,
		draft: data.draft,
		tags: data.tags,
		excerpt: data.excerpt,
		relatedPosts: data.related_posts,
		mediumShortNames: data.medium_short_names,
		mediumUrls: data.medium_urls,
		series: data.series,
		updatedAt: data.updated_at
	});
</script>

{#key data.id}
	<PostEditor mode="edit" isOwner={data.is_owner ?? true} data={mapped} />
{/key}
