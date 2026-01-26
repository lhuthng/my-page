<script>
  import { fly } from "svelte/transition";
  import { elasticInOut, elasticOut } from "svelte/easing";
  import { scrollY } from "svelte/reactivity/window";
  let { scrollTarget } = $props();
  const threshold = 300;
  let visible = $derived(scrollY.current > threshold);
</script>

{#if visible}
  <button
    in:fly={{ y: 20, duration: 800, easing: elasticOut }}
    out:fly={{ y: 20, duration: 800, easing: elasticInOut }}
    onclick={() => scrollTarget?.scrollIntoView({ behavior: "smooth" })}
    class="fixed z-50 bottom-10 right-10 w-10 h-10 rounded-full bg-white shadow-dark shadow-sm font-bold"
    title="to-top-button"
  >
    <svg class="full fill-dark p-1" viewBox="0 0 24 24">
      <g>
        <path
          fill-rule="evenodd"
          clip-rule="evenodd"
          d="M12 21C11.4477 21 11 20.5523 11 20V6.41421L5.70711 11.7071C5.31658 12.0976 4.68342 12.0976 4.29289 11.7071C3.90237 11.3166 3.90237 10.6834 4.29289 10.2929L11.2929 3.29289C11.6834 2.90237 12.3166 2.90237 12.7071 3.29289L19.7071 10.2929C20.0976 10.6834 20.0976 11.3166 19.7071 11.7071C19.3166 12.0976 18.6834 12.0976 18.2929 11.7071L13 6.41421V20C13 20.5523 12.5523 21 12 21Z"
        >
        </path>
      </g>
    </svg>
  </button>
{/if}
