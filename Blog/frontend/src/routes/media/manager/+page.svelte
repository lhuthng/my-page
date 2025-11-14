<script>
	import MediaManager from "$lib/components/MediaManager.svelte";
	let editMode = $state(true);
</script>

<MediaManager
	class="w-cap"
	{editMode}
	changeMode={() => (editMode = !editMode)}
/>
