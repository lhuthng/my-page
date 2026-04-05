<script>
  import { auth } from "$lib/client/user";
  import PostCard from "$lib/components/home/PostCard.svelte";
  import { fly, fade } from "svelte/transition";

  let { data } = $props();

  let posts = $state(data.posts ?? []);
  let total = $state(data.total ?? 0);
  let loading = $state(false);
  let loadingMore = $state(false);
  let error = $state(null);
  let search = $state("");
  let debounceTimer;
  const LIMIT = 9;

  async function fetchPosts(reset = false) {
    if (reset) {
      posts = [];
      loading = true;
    } else {
      if (loadingMore) return;
      loadingMore = true;
    }
    error = null;

    const params = new URLSearchParams({
      limit: LIMIT,
      offset: reset ? 0 : posts.length,
    });
    if (search.trim()) params.set("search", search.trim());

    try {
      const res = await fetch(`/api/dashboard/posts?${params}`, {
        headers: { Authorization: auth() },
      });
      if (!res.ok) throw new Error(await res.text());
      const data = await res.json();
      posts = reset ? data.posts : [...posts, ...data.posts];
      total = data.total;
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
      loadingMore = false;
    }
  }

  function onSearchInput() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => fetchPosts(true), 400);
  }

  let hasMore = $derived(posts.length < total);
</script>

<svelte:head>
  <title>Posts — Dashboard | Huu Thang's Blog</title>
</svelte:head>

<section class="flex flex-col gap-4 *:bg-white *:rounded-xl *:p-4 pb-8">
  <div class="flex flex-col gap-4">
    <!-- Header -->
    <div class="flex flex-wrap items-center justify-between gap-3">
      <h1 class="text-2xl font-semibold">
        Posts
        <span class="text-dark/40 text-lg font-normal">({total})</span>
      </h1>
      <div class="duo-btn duo-green w-fit">
        <a href="/dashboard/posts/new">New Post</a>
      </div>
    </div>

    <!-- Search -->
    <div
      class="flex items-center gap-2 bg-background/40 rounded-xl px-3 py-2 border border-background"
    >
      <input
        type="text"
        placeholder="Search by title or slug…"
        bind:value={search}
        oninput={onSearchInput}
        class="flex-1 bg-transparent text-base placeholder:text-dark/30 outline-none"
      />
      {#if search}
        <button
          onclick={() => {
            search = "";
            fetchPosts(true);
          }}
          class="text-dark/40 hover:text-dark text-sm cursor-pointer">✕</button
        >
      {/if}
    </div>

    <!-- Content -->
    {#if loading}
      <div class="flex justify-center items-center py-12 text-dark/40">
        Loading…
      </div>
    {:else if error}
      <p class="text-accent-red text-sm">Error: {error}</p>
    {:else if posts.length === 0}
      <div class="flex flex-col items-center gap-3 py-12 text-dark/40">
        <p class="text-lg">
          {search ? "No posts match your search" : "No posts yet"}
        </p>
        {#if !search}
          <div class="duo-btn duo-green w-fit">
            <a href="/dashboard/posts/new">Create your first post</a>
          </div>
        {/if}
      </div>
    {:else}
      <ul
        class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4"
      >
        {#each posts as post, i (post.id)}
          <li
            class="relative"
            in:fly={{ y: -20, duration: 400, delay: i * 30 }}
            out:fade={{ duration: 150 }}
          >
            <PostCard
              id={post.id}
              title={post.title}
              slug={post.slug}
              excerpt={post.excerpt}
              status={post.status}
              author={{ name: post.author_name, slug: post.author_slug }}
              tags={post.tag_slugs}
              src={post.url}
              stats={post.stats}
            />
            <!-- Edit overlay for published posts (drafts already link to edit via PostCard) -->
            {#if post.status !== "draft"}
              <a
                href="/dashboard/posts/id/{post.id}"
                class="absolute top-2 right-2 z-20 text-base bg-dark/80 text-white px-2 py-1 rounded-lg no-underline! hover:bg-dark transition-colors"
                >Edit</a
              >
            {/if}
          </li>
        {/each}
      </ul>

      {#if hasMore}
        <div class="flex justify-center bg-transparent! p-0!">
          <div class="duo-btn duo-green">
            <button onclick={() => fetchPosts(false)} disabled={loadingMore}>
              {loadingMore
                ? "Loading…"
                : `Load more (${total - posts.length} remaining)`}
            </button>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</section>
