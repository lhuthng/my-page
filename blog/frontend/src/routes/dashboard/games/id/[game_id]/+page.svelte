<script>
	import GameEditor from '$lib/components/game/GameEditor.svelte';

	const { data } = $props();

	// Reactive, not a one-time `untrack` snapshot: SvelteKit reuses this
	// component across navigations between two `/dashboard/games/id/{x}`
	// routes, so a snapshot taken once at mount would keep showing the first
	// game's data after navigating to a second one. The `{#key data.id}`
	// below is what actually forces `GameEditor` (and the view-model it
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
		demoType: data.launcher_type,
		demoWidth: data.demo_width,
		demoHeight: data.demo_height,
		rawDemoUrl: data.raw_demo_url,
		v86SystemVersionId: data.v86_system_version_id,
		v86Manifest: data.v86_manifest,
		v86ArtifactRevision: data.v86_artifact_revision,
		instruction: data.instruction,
		cheatcode: data.cheatcode,
		story: data.story,
		relatedGames: data.related_games,
		updatedAt: data.updated_at
	});
</script>

{#key data.id}
	<GameEditor
		mode="edit"
		isOwner={data.is_owner ?? true}
		v86Systems={data.v86Systems}
		games={data.games}
		data={mapped}
	/>
{/key}
