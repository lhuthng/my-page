<script>
  let { name, type, width, height } = $props();
</script>

{#if type === "html"}
  <iframe
    class="rounded-md"
    style:width
    style:height
    src={`/html/${name}/index.html`}
    title={name}
    frameborder="0"
  ></iframe>
{/if}
