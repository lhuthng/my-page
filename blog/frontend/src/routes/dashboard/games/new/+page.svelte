<script>
	import GameEditor from '$lib/components/game/GameEditor.svelte';
	let { data } = $props();
</script>

<div class="w-full">
	<GameEditor v86Systems={data.v86Systems} games={data.games} />
</div>
