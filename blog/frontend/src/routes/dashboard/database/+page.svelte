<script>
  import { goto } from "$app/navigation";
  import { navigating } from "$app/stores";

  let { data } = $props();

  // -------------------------------------------------------------------------
  // Derived from server data
  // -------------------------------------------------------------------------
  let table = $derived(data.table);
  let tableData = $derived(data.tableData);
  let dbStats = $derived(data.dbStats);
  let isLoading = $derived(!!$navigating);

  const LIMIT = 20;

  let totalPages = $derived(Math.ceil((tableData?.total ?? 0) / LIMIT));

  const TABLES = [
    { id: "users", label: "Users", statKey: "totalUsers" },
    { id: "posts", label: "Posts", statKey: "totalPosts" },
    { id: "comments", label: "Comments", statKey: "totalComments" },
    { id: "media", label: "Media", statKey: "totalMedia" },
    { id: "series", label: "Series", statKey: "totalSeries" },
    { id: "tags", label: "Tags", statKey: "totalTags" },
    { id: "categories", label: "Categories", statKey: "totalCategories" },
  ];

  // -------------------------------------------------------------------------
  // Local UI state — synced back from server data after each navigation
  // -------------------------------------------------------------------------
  let searchInput = $state(data.search || "");
  let statusFilter = $state(data.status || "");
  let roleFilter = $state(data.role || "");
  let includeDeleted = $state(data.includeDeleted || false);

  // Keep local inputs in sync when the URL changes (browser back/forward, switchTable, etc.)
  $effect(() => {
    searchInput = data.search || "";
    statusFilter = data.status || "";
    roleFilter = data.role || "";
    includeDeleted = data.includeDeleted || false;
  });

  // -------------------------------------------------------------------------
  // Navigation helper
  // -------------------------------------------------------------------------

  /**
   * Navigate to the database page with updated search params.
   * Any key passed in `params` overrides the current state; omitted keys
   * fall back to the current reactive values.
   */
  function navigate(params = {}) {
    const sp = new URLSearchParams();

    const t = params.table !== undefined ? params.table : table;
    const p = params.page !== undefined ? params.page : data.page;
    const s = params.search !== undefined ? params.search : searchInput;
    const st = params.status !== undefined ? params.status : statusFilter;
    const r = params.role !== undefined ? params.role : roleFilter;
    const id =
      params.includeDeleted !== undefined
        ? params.includeDeleted
        : includeDeleted;

    sp.set("table", t);
    sp.set("page", String(p));
    if (s) sp.set("search", s);
    if (st) sp.set("status", st);
    if (r) sp.set("role", r);
    if (id) sp.set("includeDeleted", "true");

    goto(`/dashboard/database?${sp.toString()}`);
  }

  /** Switch to a different table — resets all filters to a clean state. */
  function switchTable(id) {
    searchInput = "";
    statusFilter = "";
    roleFilter = "";
    includeDeleted = false;
    navigate({
      table: id,
      page: 1,
      search: "",
      status: "",
      role: "",
      includeDeleted: false,
    });
  }

  // -------------------------------------------------------------------------
  // Formatting helpers
  // -------------------------------------------------------------------------

  function formatBytes(bytes) {
    if (!bytes) return "0 B";
    if (bytes < 1024) return bytes + " B";
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
    return (bytes / (1024 * 1024)).toFixed(1) + " MB";
  }

  function fmtDate(iso) {
    return iso ? iso.slice(0, 10) : "—";
  }
</script>

<svelte:head>
  <title>Database | Dashboard</title>
</svelte:head>

<div class="flex gap-4 items-start">
  <!-- -----------------------------------------------------------------------
       Left sidebar — table selector
  -------- -->
  <aside
    class="w-48 shrink-0 bg-white rounded-xl p-2 flex flex-col gap-0.5 self-start sticky top-4"
  >
    <p
      class="text-xs font-semibold text-dark/40 uppercase tracking-wider px-3 py-1.5"
    >
      Tables
    </p>

    {#each TABLES as t}
      {@const count = dbStats?.[t.statKey] ?? 0}
      {@const active = table === t.id}
      <button
        onclick={() => switchTable(t.id)}
        class="flex items-center justify-between px-3 py-2 rounded-lg text-sm font-medium
               transition-colors text-left w-full
               {active
          ? 'bg-dark text-white'
          : 'text-dark hover:bg-background/60'}"
      >
        <span>{t.label}</span>
        <span
          class="text-xs tabular-nums {active
            ? 'text-white/60'
            : 'text-dark/40'}">{count}</span
        >
      </button>
    {/each}
  </aside>

  <!-- -----------------------------------------------------------------------
       Main content area
  -------- -->
  <div class="flex-1 flex flex-col gap-3 min-w-0">
    <!-- Header row: table title + search + filters --------------------------->
    <div class="bg-white rounded-xl p-4 flex flex-wrap items-center gap-3">
      <h2 class="text-xl font-semibold capitalize mr-auto">{table}</h2>

      <!-- Search — users, posts, media -->
      {#if ["users", "posts", "media"].includes(table)}
        <form
          onsubmit={(e) => {
            e.preventDefault();
            navigate({ page: 1 });
          }}
          class="flex gap-2"
        >
          <input
            bind:value={searchInput}
            placeholder="Search…"
            class="border border-background rounded-lg px-3 py-1.5 text-sm outline-none
                   focus:border-dark/30 w-44 transition-colors"
          />
          <button
            type="submit"
            class="px-3 py-1.5 bg-dark text-white rounded-lg text-sm hover:bg-dark/80
                   transition-colors"
          >
            Search
          </button>
        </form>
      {/if}

      <!-- Status filter — posts only -->
      {#if table === "posts"}
        <select
          bind:value={statusFilter}
          onchange={() => navigate({ page: 1 })}
          class="border border-background rounded-lg px-3 py-1.5 text-sm outline-none
                 bg-white cursor-pointer"
        >
          <option value="">All Status</option>
          <option value="published">Published</option>
          <option value="draft">Draft</option>
          <option value="archived">Archived</option>
        </select>
      {/if}

      <!-- Role filter — users only -->
      {#if table === "users"}
        <select
          bind:value={roleFilter}
          onchange={() => navigate({ page: 1 })}
          class="border border-background rounded-lg px-3 py-1.5 text-sm outline-none
                 bg-white cursor-pointer"
        >
          <option value="">All Roles</option>
          <option value="admin">Admin</option>
          <option value="moderator">Moderator</option>
          <option value="user">User</option>
        </select>
      {/if}

      <!-- Include-deleted toggle — comments only -->
      {#if table === "comments"}
        <label
          class="flex items-center gap-2 text-sm cursor-pointer select-none"
        >
          <input
            type="checkbox"
            bind:checked={includeDeleted}
            onchange={() => navigate({ page: 1 })}
            class="accent-primary w-4 h-4"
          />
          Show deleted
        </label>
      {/if}
    </div>

    <!-- Table card ---------------------------------------------------------->
    <div class="bg-white rounded-xl overflow-hidden">
      <!-- Record count + loading indicator -->
      <div
        class="px-4 py-2.5 border-b border-background/60 flex items-center gap-3 text-sm text-dark/50"
      >
        <span>{tableData?.total ?? 0} records</span>
        {#if isLoading}
          <span class="animate-pulse text-primary font-medium">Loading…</span>
        {/if}
      </div>

      <!-- Scrollable table wrapper -->
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <!-- Dynamic headers -->
          <thead class="bg-background/40">
            <tr>
              {#if table === "users"}
                {#each ["#", "Username", "Display Name", "Email", "Role", "Created"] as h}
                  <th
                    class="px-4 py-2.5 text-left font-semibold text-dark/50 whitespace-nowrap"
                    >{h}</th
                  >
                {/each}
              {:else if table === "posts"}
                {#each ["#", "Title", "Author", "Status", "Series", "Views", "Featured", "Created"] as h}
                  <th
                    class="px-4 py-2.5 text-left font-semibold text-dark/50 whitespace-nowrap"
                    >{h}</th
                  >
                {/each}
              {:else if table === "comments"}
                {#each ["#", "Content", "Post", "Author", "Status", "Created"] as h}
                  <th
                    class="px-4 py-2.5 text-left font-semibold text-dark/50 whitespace-nowrap"
                    >{h}</th
                  >
                {/each}
              {:else if table === "media"}
                {#each ["#", "Short Name", "File Name", "Type", "Size", "Uses", "Created"] as h}
                  <th
                    class="px-4 py-2.5 text-left font-semibold text-dark/50 whitespace-nowrap"
                    >{h}</th
                  >
                {/each}
              {:else if table === "series"}
                {#each ["#", "Title", "Slug", "Posts", "Created"] as h}
                  <th
                    class="px-4 py-2.5 text-left font-semibold text-dark/50 whitespace-nowrap"
                    >{h}</th
                  >
                {/each}
              {:else if table === "tags"}
                {#each ["#", "Name", "Slug", "Posts"] as h}
                  <th
                    class="px-4 py-2.5 text-left font-semibold text-dark/50 whitespace-nowrap"
                    >{h}</th
                  >
                {/each}
              {:else if table === "categories"}
                {#each ["#", "Name", "Slug", "Description", "Posts"] as h}
                  <th
                    class="px-4 py-2.5 text-left font-semibold text-dark/50 whitespace-nowrap"
                    >{h}</th
                  >
                {/each}
              {/if}
            </tr>
          </thead>

          <!-- Rows -->
          <tbody
            class="divide-y divide-background/60 transition-opacity duration-150 {isLoading
              ? 'opacity-40'
              : 'opacity-100'}"
          >
            {#if tableData?.items?.length}
              {#each tableData.items as row, i}
                {@const rowNum = (data.page - 1) * LIMIT + i + 1}
                <tr class="hover:bg-background/30 transition-colors">
                  <!-- Row number (shared) -->
                  <td class="px-4 py-2.5 text-dark/35 tabular-nums text-xs"
                    >{rowNum}</td
                  >

                  <!-- ── Users ────────────────────────────────────────────── -->
                  {#if table === "users"}
                    <td class="px-4 py-2.5 font-medium whitespace-nowrap"
                      >@{row.username}</td
                    >
                    <td class="px-4 py-2.5">{row.displayName}</td>
                    <td class="px-4 py-2.5 text-dark/60">{row.email}</td>
                    <td class="px-4 py-2.5">
                      <span
                        class="px-2 py-0.5 rounded-full text-xs font-medium whitespace-nowrap
                        {row.role === 'admin'
                          ? 'bg-accent-red/15 text-accent-red'
                          : row.role === 'moderator'
                            ? 'bg-accent-blue/15 text-accent-blue'
                            : 'bg-background/80 text-dark/60'}"
                      >
                        {row.role}
                      </span>
                    </td>
                    <td class="px-4 py-2.5 text-dark/45 text-xs"
                      >{fmtDate(row.createdAt)}</td
                    >

                    <!-- ── Posts ─────────────────────────────────────────────── -->
                  {:else if table === "posts"}
                    <td
                      class="px-4 py-2.5 font-medium max-w-[16rem] truncate"
                      title={row.title}>{row.title}</td
                    >
                    <td class="px-4 py-2.5 text-dark/60 whitespace-nowrap"
                      >{row.authorName ?? "—"}</td
                    >
                    <td class="px-4 py-2.5">
                      <span
                        class="px-2 py-0.5 rounded-full text-xs font-medium whitespace-nowrap
                        {row.status === 'published'
                          ? 'bg-accent-green/15 text-accent-green'
                          : row.status === 'draft'
                            ? 'bg-accent-yellow/25 text-dark/70'
                            : 'bg-background/80 text-dark/50'}"
                      >
                        {row.status}
                      </span>
                    </td>
                    <td
                      class="px-4 py-2.5 text-dark/45 text-xs max-w-40 truncate"
                      title={row.seriesTitle ?? ""}
                    >
                      {row.seriesTitle ?? "—"}
                    </td>
                    <td class="px-4 py-2.5 text-dark/60 tabular-nums"
                      >{row.viewCount}</td
                    >
                    <td class="px-4 py-2.5 text-center"
                      >{row.isFeatured ? "⭐" : "—"}</td
                    >
                    <td
                      class="px-4 py-2.5 text-dark/45 text-xs whitespace-nowrap"
                      >{fmtDate(row.createdAt)}</td
                    >

                    <!-- ── Comments ──────────────────────────────────────────── -->
                  {:else if table === "comments"}
                    <td
                      class="px-4 py-2.5 text-dark/80 max-w-[18rem] truncate"
                      title={row.content}>{row.content}</td
                    >
                    <td
                      class="px-4 py-2.5 text-dark/55 text-xs max-w-48 truncate"
                      title={row.postTitle}>{row.postTitle}</td
                    >
                    <td class="px-4 py-2.5 text-dark/60 whitespace-nowrap"
                      >{row.authorName ?? "Anonymous"}</td
                    >
                    <td class="px-4 py-2.5">
                      {#if row.isDeleted}
                        <span
                          class="px-2 py-0.5 rounded-full text-xs font-medium bg-accent-red/15 text-accent-red"
                          >Deleted</span
                        >
                      {:else}
                        <span
                          class="px-2 py-0.5 rounded-full text-xs font-medium bg-accent-green/15 text-accent-green"
                          >Active</span
                        >
                      {/if}
                    </td>
                    <td
                      class="px-4 py-2.5 text-dark/45 text-xs whitespace-nowrap"
                      >{fmtDate(row.createdAt)}</td
                    >

                    <!-- ── Media ─────────────────────────────────────────────── -->
                  {:else if table === "media"}
                    <td
                      class="px-4 py-2.5 font-mono text-xs text-dark/70 whitespace-nowrap"
                      >{row.shortName}</td
                    >
                    <td
                      class="px-4 py-2.5 text-dark/70 max-w-[16rem] truncate text-xs"
                      title={row.fileName}>{row.fileName}</td
                    >
                    <td
                      class="px-4 py-2.5 text-dark/45 text-xs whitespace-nowrap"
                      >{row.fileType}</td
                    >
                    <td
                      class="px-4 py-2.5 text-dark/60 tabular-nums text-xs whitespace-nowrap"
                      >{formatBytes(row.size)}</td
                    >
                    <td class="px-4 py-2.5 text-dark/60 tabular-nums"
                      >{row.useCount}</td
                    >
                    <td
                      class="px-4 py-2.5 text-dark/45 text-xs whitespace-nowrap"
                      >{fmtDate(row.createdAt)}</td
                    >

                    <!-- ── Series ────────────────────────────────────────────── -->
                  {:else if table === "series"}
                    <td class="px-4 py-2.5 font-medium">{row.title}</td>
                    <td class="px-4 py-2.5 font-mono text-xs text-dark/55"
                      >{row.slug}</td
                    >
                    <td class="px-4 py-2.5 text-dark/60 tabular-nums"
                      >{row.postCount}</td
                    >
                    <td
                      class="px-4 py-2.5 text-dark/45 text-xs whitespace-nowrap"
                      >{fmtDate(row.createdAt)}</td
                    >

                    <!-- ── Tags ──────────────────────────────────────────────── -->
                  {:else if table === "tags"}
                    <td class="px-4 py-2.5 font-medium">{row.name}</td>
                    <td class="px-4 py-2.5 font-mono text-xs text-dark/55"
                      >{row.slug}</td
                    >
                    <td class="px-4 py-2.5 text-dark/60 tabular-nums"
                      >{row.postCount}</td
                    >

                    <!-- ── Categories ────────────────────────────────────────── -->
                  {:else if table === "categories"}
                    <td class="px-4 py-2.5 font-medium">{row.name}</td>
                    <td class="px-4 py-2.5 font-mono text-xs text-dark/55"
                      >{row.slug}</td
                    >
                    <td
                      class="px-4 py-2.5 text-dark/45 text-xs max-w-[16rem] truncate"
                      title={row.description ?? ""}>{row.description ?? "—"}</td
                    >
                    <td class="px-4 py-2.5 text-dark/60 tabular-nums"
                      >{row.postCount}</td
                    >
                  {/if}
                </tr>
              {/each}
            {:else}
              <tr>
                <td colspan="10" class="px-4 py-12 text-center text-dark/35">
                  {isLoading ? "Loading…" : "No records found"}
                </td>
              </tr>
            {/if}
          </tbody>
        </table>
      </div>

      <!-- Pagination -------------------------------------------------------->
      {#if totalPages > 1}
        <div
          class="px-4 py-3 border-t border-background/60 flex items-center justify-between gap-4"
        >
          <button
            onclick={() => navigate({ page: data.page - 1 })}
            disabled={data.page <= 1 || isLoading}
            class="px-3 py-1.5 rounded-lg text-sm border border-background
                   disabled:opacity-40 disabled:cursor-not-allowed
                   hover:bg-background/60 transition-colors"
          >
            ← Prev
          </button>

          <span class="text-sm text-dark/55 tabular-nums">
            Page {data.page} of {totalPages}
          </span>

          <button
            onclick={() => navigate({ page: data.page + 1 })}
            disabled={data.page >= totalPages || isLoading}
            class="px-3 py-1.5 rounded-lg text-sm border border-background
                   disabled:opacity-40 disabled:cursor-not-allowed
                   hover:bg-background/60 transition-colors"
          >
            Next →
          </button>
        </div>
      {/if}
    </div>
    <!-- /table card -->
  </div>
  <!-- /main content -->
</div>
<!-- /page -->
