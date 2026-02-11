<script>
  import { onMount } from "svelte";
  import PBody from "../PBody.svelte";
  import { browser } from "$app/environment";

  let { headers } = $props();
  let ids = $derived.by(() => {
    let ids = [];
    const traverse = (array) => {
      array.forEach((element) => {
        if (Array.isArray(element)) {
          traverse(element);
        } else {
          ids.push(element.id);
        }
      });
    };
    traverse(headers);
    return ids;
  });

  let active = $state();

  function updateActive() {
    const scrollTop = window.scrollY;
    const viewportHeight = window.innerHeight;

    let firstVisible = null;

    for (const id of ids) {
      const el = document.getElementById(id);
      if (!el) continue;

      const rect = el.getBoundingClientRect();
      if (rect.bottom > 0) {
        if (!firstVisible || rect.top < firstVisible.rectTop) {
          firstVisible = { id: id, rectTop: rect.top };
        }
      }
    }

    if (firstVisible) active = firstVisible.id;
  }

  onMount(() => {
    if (!browser) return;

    updateActive();
    window.addEventListener("scroll", updateActive);

    return () => window.removeEventListener("scroll", updateActive);
  });
</script>

{#snippet toList(array)}
  <ol>
    {#each array as element}
      {#if Array.isArray(element)}
        {@render toList(element)}
      {:else}
        {@const { id, textContent } = element}
        <li>
          <a class:active={id === active} href={`#${id}`}>{textContent}</a>
        </li>
      {/if}
    {/each}
  </ol>
{/snippet}

<h2 class="text-center font-semibold text-lg pb-2">Table of contents</h2>
{@render toList(headers)}

<style lang="postcss">
  @reference "../../../app.css";

  .active {
    @apply underline text-accent-blue;
  }

  ol {
    counter-reset: item;
  }
  ol li {
    @apply block;
  }

  ol li::before {
    @apply font-bold;
    counter-increment: item;
    content: counters(item, ".") ". ";
  }

  ol ol {
    @apply pl-4;
    counter-reset: item;
  }
</style>
