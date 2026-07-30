<script>
	import ProjectEditor from '$lib/components/project/ProjectEditor.svelte';
	let { data } = $props();
</script>

<div class="w-full">
	<ProjectEditor v86Systems={data.v86Systems} />
</div>
