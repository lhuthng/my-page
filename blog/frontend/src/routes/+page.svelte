<script>
  import { page } from "$app/state";
  import HomeDiscovery from "$lib/components/home/HomeDiscovery.svelte";
  import Introduction from "$lib/components/home/Introduction.svelte";
  import Suggestion from "$lib/components/home/Suggestion.svelte";

  const { data } = $props();

  const featuredPosts = data.featured_posts || [];

  let imageUrl = $derived(page.url.origin + "/thinkcats.jpg");
</script>

<svelte:head>
  <title>Home Sweet Home | Huu Thang's Blog</title>
  <meta
    name="description"
    content="Welcome to the Field! I'm Thang, and this is my digital garden where I document work experiences, hobbies, and experiments."
  />

  <meta property="og:type" content="website" />
  <meta property="og:url" content={page.url.origin} />
  <meta property="og:title" content="Home Sweet Home | Huu Thang's Blog" />
  <meta
    property="og:description"
    content="Deep dives into technical challenges and small creative sparks."
  />
  <meta property="og:image" content={imageUrl} />

  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:image" content={imageUrl} />

  <link rel="canonical" href={page.url.href} />
</svelte:head>

<div class="relative z-5 flex gap-4 *:h-fit pb-2 lg:pb-4">
  <div class="grow space-y-2 lg:space-y-4">
    <div class="bg-white/90 rounded-xl px-4 overflow-hidden">
      <HomeDiscovery {featuredPosts} />
    </div>
  </div>
  <div class="not-lg:hidden w-60 bg-white/90 rounded-xl">
    <Suggestion />
  </div>
</div>
<div class="pb-4">
  <Introduction />
</div>

<style lang="postcss">
  @reference "../app.css";
</style>
