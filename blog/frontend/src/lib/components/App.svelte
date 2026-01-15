<script>
  import GLBDemo from "./apps/GLBDemo.svelte";
  import LottieStateSwitcher from "./apps/LottieStateSwitcher.svelte";

  let { name, type, width, height, config } = $props();
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
{:else if type === "glb-demo"}
  <div class="flex">
    <GLBDemo {name} {type} {width} {height} />
  </div>
{:else if type === "lottie"}
  {@const states = config
    .split("-")
    .reduce((a, _, i, arr) => (i % 2 ? [...a, [arr[i - 1], +arr[i]]] : a), [])}
  <LottieStateSwitcher {name} {states} {width} {height} />
{/if}
