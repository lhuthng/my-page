<script>
  import PostEditor from "$lib/components/post/PostEditor.svelte";

  const { data } = $props();
</script>

<div class="w-full">
  <PostEditor series={data.series} />
</div>
