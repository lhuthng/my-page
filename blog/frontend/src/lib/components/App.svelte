<script>
  import GLBDemo from "./apps/GLBDemo.svelte";
  import HTMLApp from "./apps/HTMLApp.svelte";
  import LottieStateSwitcher from "./apps/LottieStateSwitcher.svelte";

  let { name, type, width, height, config, temp } = $props();
</script>

{#if type === "html"}
  <HTMLApp {name} {width} {height} {config} />
{:else if type === "glb-demo"}
  <GLBDemo {name} {type} {width} {height} {config} {temp} />
{:else if type === "lottie"}
  {@const states = config
    .split("-")
    .reduce((a, _, i, arr) => (i % 2 ? [...a, [arr[i - 1], +arr[i]]] : a), [])}
  <LottieStateSwitcher {name} {states} {width} {height} />
{/if}
