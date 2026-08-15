<script>
	import ProjectEditor from '$lib/components/project/ProjectEditor.svelte';

	const { data } = $props();

	// Reactive, not a one-time `untrack` snapshot: SvelteKit reuses this
	// component across navigations between two `/dashboard/projects/id/{x}`
	// routes, so a snapshot taken once at mount would keep showing the first
	// project's data after navigating to a second one. The `{#key data.id}`
	// below is what actually forces `ProjectEditor` (and the view-model it
	// owns) to remount with fresh data when the id changes; this mapping just
	// has to stay in step with `data` in the meantime.
	const mapped = $derived({
		id: data.id,
		postId: data.post_id,
		title: data.title,
		slug: data.slug,
		coverUrl: data.cover_url,
		cover_media_type: data.cover_media_type,
		videoShortName: data.video_short_name,
		ogImageSeconds: data.og_image_seconds,
		content: data.content,
		draft: data.draft,
		tags: data.tags,
		excerpt: data.excerpt,
		mediumShortNames: data.medium_short_names,
		mediumUrls: data.medium_urls,
		demoType: data.demo_type,
		demoWidth: data.demo_width,
		demoHeight: data.demo_height,
		rawDemoUrl: data.raw_demo_url,
		v86SystemVersionId: data.v86_system_version_id,
		v86Manifest: data.v86_manifest,
		v86ArtifactRevision: data.v86_artifact_revision,
		v86GameFileName: data.v86_game_file_name,
		links: data.links,
		updatedAt: data.updated_at
	});
</script>

{#key data.id}
	<ProjectEditor
		mode="edit"
		isOwner={data.is_owner ?? true}
		v86Systems={data.v86Systems}
		data={mapped}
	/>
{/key}
