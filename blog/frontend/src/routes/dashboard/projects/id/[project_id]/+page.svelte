<script>
	import ProjectEditor from '$lib/components/project/ProjectEditor.svelte';

	const { data } = $props();

	const {
		id,
		post_id: postId,
		title,
		slug,
		cover_url: coverUrl,
		content,
		draft,
		tags,
		excerpt,
		medium_short_names: mediumShortNames,
		medium_urls: mediumUrls,
		demo_type: demoType,
		demo_width: demoWidth,
		demo_height: demoHeight,
		links,
		is_owner: isOwner = true
	} = data;
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
		content,
		draft,
		tags,
		excerpt,
		mediumShortNames,
		mediumUrls,
		demoType,
		demoWidth,
		demoHeight,
		links
	}}
/>
