<script>
	import MediaManager from '$lib/components/media/MediaManager.svelte';
	let editMode = $state(true);
</script>

<MediaManager class="w-full" {editMode} changeMode={() => (editMode = !editMode)} />
