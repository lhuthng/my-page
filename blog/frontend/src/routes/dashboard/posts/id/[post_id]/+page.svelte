<script>
	import PostEditor from '$lib/components/post/PostEditor.svelte';

	const { data } = $props();

	const {
		id,
		title,
		slug,
		series,
		cover_url: coverUrl,
		cover_media_type,
		video_short_name: videoShortName,
		og_image_seconds: ogImageSeconds,
		series_slug: seriesSlug,
		content,
		draft,
		tags,
		excerpt,
		medium_short_names: mediumShortNames,
		medium_urls: mediumUrls,
		is_owner: isOwner = true
	} = data;
</script>

<PostEditor
	mode="edit"
	{isOwner}
	data={{
		id,
		title,
		slug,
		coverUrl,
		cover_media_type,
		videoShortName,
		ogImageSeconds,
		series,
		seriesSlug,
		content,
		draft,
		tags,
		excerpt,
		mediumShortNames,
		mediumUrls
	}}
/>
