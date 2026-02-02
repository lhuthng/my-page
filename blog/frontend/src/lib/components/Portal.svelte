<script>
  import { fade } from "svelte/transition";
  let { target, visible = true, children, ...rest } = $props();
  const Portal = (element) => {
    if (!target) return;

    target.appendChild(element);
    return () => element.remove();
  };
</script>

{#if visible}
  <div
    in:fade={{ duration: 100 }}
    out:fade={{ duration: 100 }}
    {...rest}
    {@attach Portal}
  >
    {@render children?.()}
  </div>
{/if}
