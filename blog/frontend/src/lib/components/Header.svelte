<script>
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { logout, user, isMod } from "$lib/client/user";
  import { useDebounce } from "$lib/effects/debounce";
  import { fly } from "svelte/transition";
  import Portal from "$lib/components/Portal.svelte";
  import SearchButton from "./buttons/SearchButton.svelte";
  import AboutButton from "./buttons/AboutButton.svelte";
  import BlogButton from "./buttons/BlogButton.svelte";
  import DashboardButton from "./buttons/DashboardButton.svelte";
  import FacebookButton from "./buttons/FacebookButton.svelte";
  import GithubButton from "./buttons/GithubButton.svelte";
  import HomeButton from "./buttons/HomeButton.svelte";
  import LinkedinButton from "./buttons/LinkedinButton.svelte";
  import ProjectButton from "./buttons/ProjectButton.svelte";

  let displayName = $derived($user?.displayName);
  let username = $derived($user?.username);
  let role = $derived($user?.role);
  let avatarUrl = $derived($user?.avatarUrl ?? "/missing.png");
  let mainSearchContainer = $state();
  let menuSearchContainer = $state();
  let menuToggled = $state(false);

  const lg = 1024; //px
  let windowWidth = $state(0);

  $effect(() => {
    if (windowWidth > lg) menuToggled = false;
  });

  $effect(() => {
    if (menuToggled === false && windowWidth <= lg) {
      searchData._term = "";
    }
  });

  const minLength = 3;
  let searchData = $state({
    selection: "Post",
    term: "",
    _term: "",
    ignore: false,
    status: "init",
    result: null,
  });
  let termDebounce = useDebounce((_term) => {
    searchData.term = _term;
    search(searchData.selection, _term);
  }, 300);

  $effect(() => {
    if (searchData.ignore) {
      searchData.ignore = false;
      return;
    }
    termDebounce.update(searchData._term.trim());
    if (searchData._term.length < minLength) {
      searchData.status = "init";
    } else {
      searchData.status = "typing";
    }
  });

  const handleLogout = async () => {
    if ($page.url.pathname.startsWith("/dashboard")) {
      goto("/");
    }
    logout();
  };

  const change = (e) => {
    const value = e.target.value;
    searchData.selection = value;
    searchData.status = "init";
    search(value, searchData.term);
  };

  const search = async (type, term) => {
    if (term.length < minLength) {
      return;
    }
    searchData.status = "fetching";
    searchData.result = undefined;
    if (type === "User") {
      const res = await fetch(`/api/users?term=${encodeURI(term)}&size=5`, {
        method: "GET",
      });

      if (res.ok) {
        searchData.result = await res.json();
      }
    } else if (type === "Tags") {
      searchData.result = undefined;
    } else if (type === "Post") {
      const res = await fetch(`/api/posts?term=${encodeURI(term)}&size=5`, {
        method: "GET",
      });

      if (res.ok) {
        searchData.result = await res.json();
      }
    }
    searchData.status = "fetched";
  };
  const routes = [
    [HomeButton, "Home", "/", ""],
    [BlogButton, "Posts", "/posts", "posts"],
    [ProjectButton, "Projects", "/projects", "projects"],
    [AboutButton, "About", "/about", "about"],
    [DashboardButton, "Dashboard", "/dashboard", "dashboard", true],
  ];
</script>

<svelte:window bind:innerWidth={windowWidth} />

<header class="fixed w-full bg-white text-dark z-100">
  <div
    class="z-10 flex not-lg:justify-center w-cap-2 p-2 lg:p-4 gap-4 shadow-lg"
  >
    <div class="flex items-center gap-4">
      <div
        class="rounded-full bg-background transition-all duration-200 shadow-lg hover:brightness-102 hover:scale-102"
      >
        <a href="/"
          ><img class="not-lg:h-10 h-20" src={"/logo.svg"} alt="logo icon" /></a
        >
      </div>
    </div>
    <div class="relative grow not-lg:hidden">
      <a
        class="font-semibold text-3xl text-dark transition-all duration-200 hover:scale-102"
        href="/">Huu Thang's blog</a
      >
      <div
        class="relative z-10 flex max-w-160 h-10 items-center rounded-xl border-2 transition-colors duration-50 bg-dark border-dark overflow-hidden"
      >
        <div class="relative grow h-full grid px-2 bg-white">
          <input
            class="w-full"
            name="search-input"
            placeholder="Search"
            autocomplete="off"
            bind:value={searchData._term}
          />
          <button
            title="close-btn"
            class="absolute right-2 top-1/2 -translate-y-1/2 h-full w-fit"
            onclick={() => {
              searchData._term = "";
              searchData.term = "";
              searchData.status = "init";
            }}
          >
            <svg class="h-3/5 stroke-dark" viewBox="0 0 24 24">
              <path
                id="primary"
                d="M19,19,5,5M19,5,5,19"
                style="stroke-linecap: round; stroke-linejoin: round; stroke-width: 2;"
              />
            </svg>
          </button>
        </div>
        <div class="flex items-center h-full">
          <div class="h-full px-2 border-l-2 rounded-r-lg bg-white">
            <select
              class="full text-dark outline-none"
              name="search-options"
              onchange={change}
            >
              <option>Post</option>
              <option>Tag</option>
              <option>User</option>
            </select>
          </div>

          <div>
            <SearchButton
              class="p-2 w-10 h-full transition-transform duration-50 active:translate-y-0.5"
              fill="white"
              onclick={() => search(searchData.selection, searchData._term)}
            />
          </div>
        </div>
      </div>
      <div
        class="absolute z-9 left-0 w-full top-full h-fit drop-shadow-sm"
        bind:this={mainSearchContainer}
      ></div>
    </div>
    <div class="flex flex-col not-lg:hidden items-end gap-3">
      <div class="flex h-9 gap-2 items-center text-dark">
        {#if displayName && username}
          <div class="duo-btn duo-primary">
            <button onclick={handleLogout}>Sign Out</button>
          </div>
        {:else}
          <span>wanna</span>
          <div class="duo-btn duo-primary">
            <a class="no-underline!" href="/login"> Log In </a>
          </div>
          <span class="text-dark">or</span>
          <div class="duo-btn duo-primary">
            <a class="no-underline!" href="/login?register=true"> Sign Up? </a>
          </div>
        {/if}
      </div>
      {#if displayName && username}
        <div>
          Welcome back,
          <a class="font-bold" href="/profiles/{username}">
            {displayName}
            <img
              class="w-6 h-6 ml-1 rounded-full border-2 inline object-cover"
              src={avatarUrl}
              alt="small-avatar"
            />
          </a>
        </div>
      {/if}
    </div>
  </div>
  <div class="lg:hidden absolute right-4 top-1/2 -translate-y-1/2">
    <button
      type="button"
      aria-label="Open Mobile Menu"
      onclick={() => (menuToggled = !menuToggled)}
    >
      <svg
        class="w-6 h-6 stroke-dark stroke-2 *:transition-transform *:duration-200 *:origin-center *:transform-border"
        viewBox="0 0 24 24"
        x="0px"
        y="0px"
      >
        <path
          class:translate-y-1.5={menuToggled}
          class:rotate-45={menuToggled}
          d="M4 6H20"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          class:scale-x-0={menuToggled}
          d="M4 12H20"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          class:-translate-y-1.5={menuToggled}
          class:-rotate-45={menuToggled}
          d="M4 18H20"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
  </div>
  {#if menuToggled}
    <div
      in:fly={{ y: -10, duration: 200 }}
      out:fly={{ y: -10, duration: 200 }}
      id="mobile-menu"
      class="z-9 absolute flex flex-col gap-4 items-center w-cap bg-white py-4 rounded-b-lg shadow-lg"
    >
      {#if displayName && username}
        <div class="flex not-sm:flex-col items-center gap-2">
          <div>
            <span>Welcome back,</span>
            <a class="font-bold" href="/profiles/{username}">
              {displayName}
              <img
                class="w-6 h-6 ml-1 rounded-full border-2 inline object-cover"
                src={avatarUrl}
                alt="small-avatar"
              />
            </a>
          </div>
          <div class="duo-btn duo-primary">
            <button onclick={handleLogout}>Sign Out</button>
          </div>
        </div>
      {/if}
      {#if !displayName || !username}
        <div class="relative flex h-9 items-center">
          <div class="absolute right-full pr-2">
            <div class="duo-btn w-fit duo-primary">
              <a
                class="no-underline!"
                href="/login"
                onclick={() => (menuToggled = false)}
              >
                log in
              </a>
            </div>
          </div>
          <span class="text-dark">or</span>
          <div class="absolute left-full pl-2">
            <div class="duo-btn w-fit duo-primary">
              <a
                class="no-underline!"
                href="/login?register=true"
                onclick={() => (menuToggled = false)}
              >
                sign up?
              </a>
            </div>
          </div>
        </div>
      {/if}
      <div class="w-full flex not-md:flex-col">
        <div class="flex flex-col gap-2 grow">
          <div
            class="relative z-10 flex w-full h-10 items-center rounded-xl border-2 transition-colors duration-50 bg-dark border-dark overflow-hidden"
          >
            <div class="relative grow h-full grid px-2 bg-white">
              <input
                class="w-full"
                name="search-input"
                placeholder="Search"
                autocomplete="off"
                bind:value={searchData._term}
              />
              <button
                title="close-btn"
                class="absolute right-2 top-1/2 -translate-y-1/2 h-full w-fit"
                onclick={() => {
                  searchData._term = "";
                  searchData.term = "";
                  searchData.status = "init";
                }}
              >
                <svg class="h-3/5 stroke-dark" viewBox="0 0 24 24">
                  <path
                    id="primary"
                    d="M19,19,5,5M19,5,5,19"
                    style="stroke-linecap: round; stroke-linejoin: round; stroke-width: 2;"
                  />
                </svg>
              </button>
            </div>
            <div class="min-w-fit flex items-center h-full">
              <div class="h-full px-2 border-l-2 rounded-r-lg bg-white">
                <select
                  class="full text-dark outline-none"
                  name="search-options"
                  onchange={change}
                >
                  <option>Post</option>
                  <option>Tag</option>
                  <option>User</option>
                </select>
              </div>

              <div>
                <SearchButton
                  class="p-2 w-10 h-full transition-transform duration-50 active:translate-y-0.5"
                  fill="white"
                  onclick={() => search(searchData.selection, searchData._term)}
                />
              </div>
            </div>
          </div>
          {#if searchData.status !== "init"}
            <div class="w-full" bind:this={menuSearchContainer}></div>
          {/if}
        </div>
        <div class="w-1/2 flex not-md:flex-col justify-evenly">
          <ul class="space-y-2 bg-white/90 p-2 rounded-xl" id="side-bar">
            {#each routes as [Icon, text, path, routeName, secret], index}
              {#if !secret || $isMod}
                <li>
                  <a
                    class="flex items-center gap-2 stroke-dark fill-dark"
                    href={path}
                  >
                    <Icon class="w-6 transition-all duration-100" />
                    <span>{text}</span>
                  </a>
                </li>
              {/if}
            {/each}
          </ul>
          <div class="flex flex-col p-2">
            <span>Connect with me on:</span>
            <ul
              class="flex flex-col gap-1 [&>li]:flex [&>li]:gap-1 [&>li>a]:hover:fill-primary [&>li>a]:w-7"
            >
              <li>
                <FacebookButton
                  as="a"
                  class="fill-dark"
                  href="https://www.facebook.com/lhuthng/"
                />
                <a href="https://www.facebook.com/lhuthng/">Facebook</a>
              </li>
              <li>
                <GithubButton
                  as="a"
                  class="fill-dark"
                  href="https://github.com/lhuthng"
                />
                <a href="https://github.com/lhuthng">Github</a>
              </li>
              <li>
                <LinkedinButton
                  as="a"
                  class="fill-dark"
                  href="https://www.linkedin.com/in/huuthangle/"
                />
                <a href="https://www.linkedin.com/in/huuthangle/">Linkedin</a>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  {/if}
</header>

{#if searchData.status !== "init"}
  <Portal target={menuToggled ? menuSearchContainer : mainSearchContainer}>
    <div
      in:fly={{ y: -10, duration: 200 }}
      out:fly={{ y: 10, duration: 200 }}
      class="w-full h-fit bg-white rounded-b-xl px-2 pb-2 lg:p-4"
    >
      {#if searchData.status === "fetching" || searchData.status === "typing"}
        <span>Loading</span>
      {:else if searchData.status === "fetched"}
        {#if searchData.selection === "User"}
          {#if searchData.result === null || searchData.result.users.length === 0}
            <span>
              No user matches "{searchData.term}"
            </span>
          {:else}
            <ul>
              {#each searchData.result.users as { username, display_name: displayName, role, avatar_url: avatarUrl }, index (username)}
                <li class="flex gap-4 items-center">
                  <img
                    class="w-10 h-10 object-cover rounded-sm"
                    src={avatarUrl ?? "/missing.png"}
                    alt="mini-avatar"
                  />
                  <a href={"/profiles/" + username}>{displayName}</a>
                </li>
              {/each}
            </ul>
          {/if}
        {:else if searchData.selection === "Tag"}
          <span
            >Tag searching is not supported (yet) <span
              class="text-nowrap font-bold">(づ_ど)</span
            >
          </span>
        {:else if searchData.selection === "Post"}
          {#if searchData.result === null || searchData.result.posts.length === 0}
            <span>
              No post matches "{searchData.term}"
            </span>
          {:else}
            <ul class="space-y-2">
              {#each searchData.result.posts as { title, slug, cover_url: coverUrl }, index (slug)}
                <li class="flex gap-4 items-center">
                  <img
                    class="w-10 h-10 object-cover rounded-sm"
                    src={coverUrl ?? "/missing.png"}
                    alt="mini-avatar"
                  />
                  <a href={"posts/" + slug}>{title}</a>
                </li>
              {/each}
            </ul>
          {/if}
        {/if}
      {/if}
    </div>
  </Portal>
{/if}

<style lang="postcss">
  @reference "../../app.css";
</style>
