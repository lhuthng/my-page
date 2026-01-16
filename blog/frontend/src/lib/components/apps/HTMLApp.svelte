<script>
  import { fade, fly } from "svelte/transition";

  let { name, width, height, config } = $props();

  let loaded = $state(false);
</script>

<div
  class="relative mx-auto bg-background rounded-lg"
  style:width
  style:max-width={width}
  style:height
>
  {#if loaded}
    <iframe
      in:fade={{ duration: 800 }}
      class="rounded-md"
      style:width
      style:height
      src={`/html/${name}/index.html`}
      title={name}
      frameborder="0"
    >
    </iframe>
  {:else}
    <div
      out:fly={{ y: 20, duration: 800 }}
      class="absolute top-1/2 left-1/2 -translate-1/2"
    >
      <div class="w-fit duo-btn duo-green">
        <button onclick={() => (loaded = true)}>Start App</button>
      </div>
    </div>
  {/if}
</div>
