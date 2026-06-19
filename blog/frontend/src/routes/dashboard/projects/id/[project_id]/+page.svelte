<script>
	import { untrack } from 'svelte';
	import ProjectEditor from '$lib/components/project/ProjectEditor.svelte';

	const { data } = $props();

	const {
		id,
		post_id: postId,
		title,
		slug,
		cover_url: coverUrl,
		cover_media_type,
		video_short_name: videoShortName,
		og_image_seconds: ogImageSeconds,
		content,
		draft,
		tags,
		excerpt,
		medium_short_names: mediumShortNames,
		medium_urls: mediumUrls,
		demo_type: demoType,
		demo_width: demoWidth,
		demo_height: demoHeight,
		raw_demo_url: rawDemoUrl,
		links,
		is_owner: isOwner = true
	} = untrack(() => data);
</script>

<ProjectEditor
	mode="edit"
	{isOwner}
	data={{
		id,
		postId,
		title,
		slug,
		coverUrl,
		cover_media_type,
		videoShortName,
		ogImageSeconds,
		content,
		draft,
		tags,
		excerpt,
		mediumShortNames,
		mediumUrls,
		demoType,
		demoWidth,
		demoHeight,
		rawDemoUrl,
		links
	}}
/>
