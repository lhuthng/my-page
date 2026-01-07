<script>
  import { pbody } from "$lib/client/misc";
  import Portal from "./Portal.svelte";

  let { children } = $props();
</script>

<Portal
  class={`full bg-dark/50 block overflow-hidden ${!children ? "opacity-0 pointer-events-none" : "opacity-100 pointer-events-auto"}`}
  target={$pbody}
>
  {@render children?.()}
</Portal>
