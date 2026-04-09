<script>
  import { page } from "$app/stores";

  let { children } = $props();

  let currentPath = $derived($page.url.pathname);

  const baseTabs = [
    { label: "Overview", path: "/dashboard" },
    { label: "Posts", path: "/dashboard/posts" },
    { label: "Series", path: "/dashboard/series" },
    { label: "Users", path: "/dashboard/users" },
  ];

  // $page.data.role is returned by +layout.server.js on every navigation
  // (both SSR and client-side), so it's always reliable. $page.data.user
  // is only populated on the initial HTML request (set via hooks.server.js)
  // and becomes null on subsequent client-side navigations.
  let tabs = $derived(
    $page.data.role === "admin"
      ? [...baseTabs, { label: "Database", path: "/dashboard/database" }]
      : baseTabs,
  );
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
