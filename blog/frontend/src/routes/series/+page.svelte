<script>
  import PostCard from "$lib/components/home/PostCard.svelte";

  let { data } = $props();

  let series = $derived(data.series);
</script>

{#snippet postItem({
  title,
  slug,
  excerpt,
  author_name,
  author_slug,
  tag_slugs,
  url,
  stats,
})}
  <li class="h-fit">
    <PostCard
      {title}
      {slug}
      {excerpt}
      author={{ name: author_name, slug: author_slug }}
      tags={tag_slugs}
      src={url}
      {stats}
    />
  </li>
{/snippet}

{#snippet seriesItem({
  title,
  display_name,
  username,
  slug,
  description,
  url,
  posts,
})}
  <li
    class="flex not-lg:flex-col gap-2 bg-background/40 *:bg-primary/20 p-2 rounded-2xl *:p-2 *:rounded-2xl max-h-160"
  >
    <div class="flex flex-col gap-4 lg:max-w-60">
      <img
        class="mt-4 mx-auto w-full max-w-40 max-h-40 rounded-xl object-cover"
        src={url ?? "/missing.png"}
        alt={`series-cover-of-${slug}`}
      />
      <div class="text-center">
        <h2 class="text-xl font-semibold text-center">{title}</h2>
        <span class="text-base"
          ><span class="opacity-60">By{" "}</span><a
            href={"/profiles/" + username}>@{display_name}</a
          ></span
        >
      </div>

      <p>{description}</p>
    </div>
    <div class="grow max-h-full overflow-y-scroll custom-scrollbar">
      <ol
        class="w-full h-fit grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-2"
      >
        {#each posts as post}
          {@render postItem(post)}
        {/each}
      </ol>
    </div>
  </li>
{/snippet}

<svelte:head>
  <title>Series | Huu Thang's Blog</title>
  <meta property="og:title" content="Posts" />
</svelte:head>

<section class="flex flex-col gap-4 *:bg-white *:rounded-xl *:p-4 pb-8">
  <div>
    <h1 class="text-2xl font-semibold">Series</h1>
    <ul class="flex flex-col">
      {#each series as series}
        {@render seriesItem(series)}
      {/each}
    </ul>
  </div>
</section>
