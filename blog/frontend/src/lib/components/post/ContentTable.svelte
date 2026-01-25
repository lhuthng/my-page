<script>
  let { headers } = $props();

  $effect(() => {
    $inspect(headers);
  });
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

<div class="bg-white text-base rounded-xl p-2">
  <h2 class="text-center font-semibold text-lg">Table of contents</h2>
  {@render toList(headers)}
</div>

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
