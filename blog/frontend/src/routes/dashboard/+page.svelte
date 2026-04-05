<script>
  let { data } = $props();

  let overview = $derived(data.overview);
  let activeTopTab = $state("views");

  let topPosts = $derived(
    activeTopTab === "views"
      ? overview?.top_posts_by_views
      : activeTopTab === "likes"
        ? overview?.top_posts_by_likes
        : overview?.top_posts_by_comments,
  );

  function roleLabel(role) {
    return role === "admin" ? "Admin" : role === "moderator" ? "Mod" : "User";
  }
</script>

<svelte:head>
  <title>Dashboard | Huu Thang's Blog</title>
</svelte:head>

<section class="flex flex-col gap-4 pb-8">
  {#if overview}
    <!-- ── Stat cards ─────────────────────────────── -->
    <div class="grid grid-cols-2 xl:grid-cols-4 gap-4">
      {#each [["Published Posts", overview.total_published, "text-accent-green"], ["Drafts", overview.total_drafts, "text-accent-yellow"], ["Registered Users", overview.total_users, "text-accent-blue"], ["Comments", overview.total_comments, "text-primary"]] as [label, value, color]}
        <div class="bg-white rounded-xl p-4">
          <p class="text-3xl font-bold {color}">{value}</p>
          <p class="text-base text-dark/60 mt-1">{label}</p>
        </div>
      {/each}
    </div>

    <!-- ── Top posts + Role breakdown ─────────────── -->
    <div class="grid xl:grid-cols-3 gap-4">
      <!-- Top posts (takes 2/3 width on xl) -->
      <div class="xl:col-span-2 bg-white rounded-xl p-4 flex flex-col gap-3">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <h2 class="text-2xl font-semibold">Top Performing Posts</h2>
          <div class="flex gap-1 bg-background/40 rounded-lg p-1 text-base">
            {#each [["views", "Views"], ["likes", "Likes"], ["comments", "Comments"]] as [key, label]}
              <button
                onclick={() => (activeTopTab = key)}
                class="px-3 py-1 rounded-md transition-colors cursor-pointer {activeTopTab ===
                key
                  ? 'bg-white font-semibold shadow-sm text-dark'
                  : 'text-dark/60 hover:text-dark'}">{label}</button
              >
            {/each}
          </div>
        </div>
        {#if topPosts?.length}
          <ol class="flex flex-col divide-y divide-background">
            {#each topPosts as post, i}
              <li
                class="flex items-center gap-3 py-2 first:pt-0 last:pb-0 hover:bg-background/20 transition-colors rounded-lg px-2 -mx-2"
              >
                <span
                  class="text-dark/30 font-bold text-base w-6 text-center shrink-0"
                  >{i + 1}</span
                >
                <div class="flex-1 min-w-0">
                  <a
                    href="/posts/{post.slug}"
                    class="font-medium text-base truncate block hover:text-primary"
                    >{post.title}</a
                  >
                  <span class="text-sm text-dark/50">by {post.author_name}</span
                  >
                </div>
                <span class="text-base font-semibold text-primary shrink-0">
                  {activeTopTab === "views"
                    ? post.stats.views
                    : activeTopTab === "likes"
                      ? post.stats.likes
                      : post.stats.comments}
                </span>
              </li>
            {/each}
          </ol>
        {:else}
          <p class="text-dark/40 text-sm text-center py-4">
            No published posts yet
          </p>
        {/if}
      </div>

      <!-- Role breakdown (1/3) -->
      <div class="bg-white rounded-xl p-4 flex flex-col gap-4">
        <h2 class="text-2xl font-semibold">User Roles</h2>
        {#if overview.role_counts}
          {@const total = Math.max(
            overview.role_counts.admin +
              overview.role_counts.moderator +
              overview.role_counts.user,
            1,
          )}
          <div class="flex flex-col gap-3 grow">
            {#each [["Admins", overview.role_counts.admin, "bg-accent-red"], ["Moderators", overview.role_counts.moderator, "bg-accent-blue"], ["Users", overview.role_counts.user, "bg-accent-green"]] as [label, count, color]}
              <div class="flex flex-col gap-1">
                <div class="flex justify-between text-base">
                  <span class="font-medium">{label}</span>
                  <span class="text-dark/60">{count}</span>
                </div>
                <div
                  class="h-1.5 bg-background/50 rounded-full overflow-hidden"
                >
                  <div
                    class="h-full rounded-full {color} transition-all duration-500"
                    style="width:{Math.round((count / total) * 100)}%"
                  ></div>
                </div>
              </div>
            {/each}
          </div>
          <p
            class="text-sm text-dark/40 border-t border-background pt-3 text-center"
          >
            {overview.total_users} users total
          </p>
        {/if}
      </div>
    </div>

    <!-- ── Growth chart ────────────────────────────── -->
    {#if overview.growth?.length}
      {@const maxVal = Math.max(
        ...overview.growth.flatMap((g) => [g.new_posts, g.new_users]),
        1,
      )}
      <div class="bg-white rounded-xl p-4 flex flex-col gap-3">
        <h2 class="text-2xl font-semibold">Activity — Last 30 Days</h2>
        <div class="flex gap-4 text-sm text-dark/60">
          <span class="flex items-center gap-1.5">
            <span class="inline-block w-3 h-3 rounded-sm bg-accent-blue"></span>
            New Posts
          </span>
          <span class="flex items-center gap-1.5">
            <span class="inline-block w-3 h-3 rounded-sm bg-accent-green"
            ></span>
            New Users
          </span>
        </div>
        <div class="flex items-end gap-px h-24 overflow-x-auto">
          {#each overview.growth as g}
            <div
              class="flex flex-col min-w-3 flex-1 h-full justify-end group relative"
            >
              <!-- Tooltip -->
              <div
                class="absolute bottom-full mb-1 hidden group-hover:block bg-dark text-white text-sm rounded px-2 py-1 whitespace-nowrap z-10 pointer-events-none left-1/2 -translate-x-1/2"
              >
                {g.date}: {g.new_posts} posts · {g.new_users} users
              </div>
              <div class="w-full flex gap-px items-end h-full">
                <div
                  class="flex-1 bg-accent-blue rounded-t-sm"
                  style="height:{g.new_posts
                    ? Math.max(Math.round((g.new_posts / maxVal) * 100), 2)
                    : 0}%"
                ></div>
                <div
                  class="flex-1 bg-accent-green rounded-t-sm"
                  style="height:{g.new_users
                    ? Math.max(Math.round((g.new_users / maxVal) * 100), 2)
                    : 0}%"
                ></div>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- ── Recent posts + Recent users ────────────── -->
    <div class="grid xl:grid-cols-2 gap-4">
      <!-- Recent Posts -->
      <div class="bg-white rounded-xl p-4 flex flex-col gap-3">
        <div class="flex items-center justify-between">
          <h2 class="text-2xl font-semibold">Recent Posts</h2>
          <a
            href="/dashboard/posts"
            class="text-base text-primary hover:underline">View all →</a
          >
        </div>
        {#if overview.recent_posts?.length}
          <ul class="flex flex-col divide-y divide-background">
            {#each overview.recent_posts as post}
              <li class="flex items-center gap-3 py-2 first:pt-0 last:pb-0">
                {#if post.url}
                  <img
                    src={post.url}
                    alt=""
                    class="w-10 h-10 rounded-lg object-cover shrink-0"
                  />
                {:else}
                  <div
                    class="w-10 h-10 rounded-lg bg-background/60 shrink-0"
                  ></div>
                {/if}
                <div class="flex-1 min-w-0">
                  <a
                    href="/dashboard/posts"
                    class="text-base font-medium truncate block hover:text-primary"
                    >{post.title}</a
                  >
                  <div class="flex items-center gap-2 text-sm text-dark/50">
                    <span>by {post.author_name}</span>
                    <span
                      class="px-1.5 py-0.5 rounded-full {post.status ===
                      'published'
                        ? 'bg-accent-green/20 text-accent-green'
                        : 'bg-accent-yellow/30 text-dark/70'}"
                      >{post.status}</span
                    >
                  </div>
                </div>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="text-dark/40 text-sm text-center py-4">No posts yet</p>
        {/if}
      </div>

      <!-- Recent Users -->
      <div class="bg-white rounded-xl p-4 flex flex-col gap-3">
        <div class="flex items-center justify-between">
          <h2 class="text-2xl font-semibold">Recent Registrations</h2>
          <a
            href="/dashboard/users"
            class="text-base text-primary hover:underline">View all →</a
          >
        </div>
        {#if overview.recent_users?.length}
          <ul class="flex flex-col divide-y divide-background">
            {#each overview.recent_users as u}
              <li class="flex items-center gap-3 py-2 first:pt-0 last:pb-0">
                {#if u.avatar_url}
                  <img
                    src={u.avatar_url}
                    alt=""
                    class="w-8 h-8 rounded-full object-cover shrink-0"
                  />
                {:else}
                  <div
                    class="w-8 h-8 rounded-full bg-primary/20 flex items-center justify-center text-sm font-bold text-primary shrink-0"
                  >
                    {u.display_name.charAt(0).toUpperCase()}
                  </div>
                {/if}
                <div class="flex-1 min-w-0">
                  <p class="text-base font-medium truncate">{u.display_name}</p>
                  <p class="text-sm text-dark/50">@{u.username}</p>
                </div>
                <span
                  class="text-sm px-2 py-0.5 rounded-full shrink-0 {u.role ===
                  'admin'
                    ? 'bg-accent-red/20 text-accent-red'
                    : u.role === 'moderator'
                      ? 'bg-accent-blue/20 text-accent-blue'
                      : 'bg-background/60 text-dark/60'}"
                >
                  {roleLabel(u.role)}
                </span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="text-dark/40 text-sm text-center py-4">No users yet</p>
        {/if}
      </div>
    </div>
  {:else}
    <div class="bg-white rounded-xl p-8 flex justify-center text-dark/40">
      No data available.
    </div>
  {/if}
</section>
