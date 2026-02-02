<script>
  import PBody from "../PBody.svelte";

  let { headers } = $props();
</script>

{#snippet toList(array)}
  <ol>
    {#each array as element}
      {#if Array.isArray(element)}
        {@render toList(element)}
      {:else}
        {@const { id, textContent } = element}
        <li><a href={`#${id}`}>{textContent}</a></li>
      {/if}
    {/each}
  </ol>
{/snippet}

<h2 class="text-center font-semibold text-lg pb-2">Table of contents</h2>
{@render toList(headers)}

<style lang="postcss">
  @reference "../../../app.css";

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
