<script>
  import { auth } from "$lib/client/user";
  import { fly, fade } from "svelte/transition";

  let { data } = $props();

  let series = $state(data.series ?? []);

  // Per-series expanded state and posts
  let expanded = $state({}); // { [id]: boolean }
  let seriesPosts = $state({}); // { [id]: { status, items } }

  const toggleExpand = async (id) => {
    expanded[id] = !expanded[id];
    if (expanded[id] && !seriesPosts[id]) {
      await loadPosts(id);
    }
  };

  const loadPosts = async (seriesId) => {
    seriesPosts[seriesId] = { status: "loading", items: [] };
    try {
      const res = await fetch(`/api/series/id/${seriesId}/posts`, {
        headers: { Authorization: auth() },
      });
      if (!res.ok) {
        seriesPosts[seriesId] = { status: "error", items: [] };
        return;
      }
      const { posts } = await res.json();
      seriesPosts[seriesId] = { status: "ok", items: posts };
    } catch {
      seriesPosts[seriesId] = { status: "error", items: [] };
    }
  };

  const removePost = async (seriesId, postId) => {
    const res = await fetch(`/api/series/id/${seriesId}?post_id=${postId}`, {
      method: "DELETE",
      headers: { Authorization: auth() },
    });
    if (res.ok) {
      // Remove from local state
      const entry = seriesPosts[seriesId];
      if (entry) {
        entry.items = entry.items.filter((p) => p.post_id !== postId);
      }
      // Decrement count in the series list
      const s = series.find((s) => s.id === seriesId);
      if (s) s.post_count = Math.max(0, s.post_count - 1);
    }
  };
</script>

<svelte:head>
  <title>Series — Dashboard | Huu Thang's Blog</title>
</svelte:head>

<section class="flex flex-col gap-4 pb-8">
  <div class="bg-white rounded-xl p-4 flex items-center justify-between">
    <h1 class="text-2xl font-semibold">
      Series
      <span class="text-dark/40 text-lg font-normal">({series.length})</span>
    </h1>
  </div>

  {#if series.length === 0}
    <div class="bg-white rounded-xl p-8 text-center text-dark/40">
      <p class="text-lg">No series yet.</p>
      <p class="text-sm mt-1">Create a series from the post editor.</p>
    </div>
  {:else}
    <ul class="flex flex-col gap-3">
      {#each series as s (s.id)}
        <li
          class="bg-white rounded-xl overflow-hidden"
          in:fly={{ y: -10, duration: 300 }}
        >
          <!-- Series header row -->
          <div class="flex items-center gap-4 p-4">
            {#if s.url}
              <img
                src={s.url}
                alt="series cover"
                class="w-16 h-16 rounded-lg object-cover shrink-0"
              />
            {:else}
              <div class="w-16 h-16 rounded-lg bg-primary/20 shrink-0"></div>
            {/if}

            <div class="flex flex-col min-w-0 grow">
              <div class="flex items-center gap-2 flex-wrap">
                <h2 class="font-semibold text-lg">{s.title}</h2>
                {#if s.owner_username}
                  <span
                    class="text-xs bg-primary/20 px-2 py-0.5 rounded-full text-dark/60"
                  >
                    by {s.owner_username}
                  </span>
                {/if}
              </div>
              <span class="text-sm text-dark/50 font-mono">/{s.slug}</span>
              {#if s.description}
                <p class="text-sm text-dark/70 mt-1 line-clamp-2">
                  {s.description}
                </p>
              {/if}
            </div>

            <div class="flex items-center gap-3 shrink-0">
              <span class="text-sm text-dark/50"
                >{s.post_count} post{s.post_count !== 1 ? "s" : ""}</span
              >
              <div
                class="duo-btn"
                class:duo-green={!expanded[s.id]}
                class:duo-red={expanded[s.id]}
              >
                <button onclick={() => toggleExpand(s.id)}>
                  {expanded[s.id] ? "Collapse" : "Manage"}
                </button>
              </div>
            </div>
          </div>

          <!-- Expanded posts section -->
          {#if expanded[s.id]}
            <div
              class="border-t border-background px-4 pb-4"
              in:fly={{ y: -8, duration: 200 }}
            >
              {#if seriesPosts[s.id]?.status === "loading"}
                <p class="py-4 text-center text-dark/40">Loading posts…</p>
              {:else if seriesPosts[s.id]?.status === "error"}
                <p class="py-4 text-center text-accent-red">
                  Failed to load posts.
                </p>
              {:else if seriesPosts[s.id]?.items.length === 0}
                <p class="py-4 text-center text-dark/40">
                  No posts in this series yet.
                </p>
              {:else}
                <ul class="flex flex-col divide-y divide-background">
                  {#each seriesPosts[s.id].items as post (post.post_id)}
                    <li
                      class="flex items-center gap-3 py-2"
                      out:fade={{ duration: 150 }}
                    >
                      <span
                        class="text-dark/40 text-sm w-6 text-right shrink-0"
                      >
                        #{post.number}
                      </span>
                      {#if post.cover_url}
                        <img
                          src={`/api/${post.cover_url}`.replace("/./", "/")}
                          alt="cover"
                          class="w-10 h-10 rounded object-cover shrink-0"
                        />
                      {:else}
                        <div
                          class="w-10 h-10 rounded bg-primary/20 shrink-0"
                        ></div>
                      {/if}
                      <div class="flex flex-col min-w-0 grow">
                        <a
                          href="/dashboard/posts/id/{post.post_id}"
                          class="font-medium text-sm line-clamp-1 no-underline! text-dark hover:underline"
                        >
                          {post.title}
                        </a>
                        <span class="text-xs text-dark/50">{post.slug}</span>
                      </div>
                      <span
                        class="text-xs px-2 py-0.5 rounded-full shrink-0 {post.status ===
                        'published'
                          ? 'bg-accent-green text-white'
                          : 'bg-primary/20 text-dark'}"
                      >
                        {post.status}
                      </span>
                      <button
                        class="text-accent-red text-sm hover:underline shrink-0"
                        onclick={() => removePost(s.id, post.post_id)}
                      >
                        Remove
                      </button>
                    </li>
                  {/each}
                </ul>
              {/if}
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>
