<script>
  let { children } = $props();
</script>

<svelte:head>
  <meta name="robots" content="noindex, nofollow" />
</svelte:head>

{#if children !== undefined}
  {@render children()}
{/if}
