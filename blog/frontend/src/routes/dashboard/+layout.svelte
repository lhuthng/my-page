<script>
  import { page } from "$app/stores";

  let { children } = $props();

  let currentPath = $derived($page.url.pathname);

  const tabs = [
    { label: "Overview", path: "/dashboard" },
    { label: "Posts", path: "/dashboard/posts" },
    { label: "Users", path: "/dashboard/users" },
  ];
</script>

<svelte:head>
  <meta name="robots" content="noindex, nofollow" />
</svelte:head>

<div class="flex flex-col gap-2 lg:gap-4 pb-8">
  <nav class="bg-white rounded-xl p-2 flex gap-1">
    {#each tabs as tab}
      {@const active =
        currentPath === tab.path ||
        (tab.path !== "/dashboard" && currentPath.startsWith(tab.path))}
      <a
        href={tab.path}
        class="px-4 py-1.5 rounded-lg text-lg font-medium transition-colors no-underline! {active
          ? 'bg-dark text-white'
          : 'text-dark hover:bg-background/60'}"
      >
        {tab.label}
      </a>
    {/each}
  </nav>
  {@render children?.()}
</div>
