<script>
  import { afterNavigate, goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { logout, user, isMod } from "$lib/client/user";
  import { useDebounce } from "$lib/effects/debounce";
  import { draw, fly } from "svelte/transition";
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
  import PBody from "./PBody.svelte";
  import { quadInOut } from "svelte/easing";
  import { onMount } from "svelte";
  import SeriesButton from "./buttons/SeriesButton.svelte";
  import { widthThreshold } from "$lib/common";
  import { isLg } from "$lib/client/windows";
  const { lg } = widthThreshold;

  let displayName = $derived($user?.displayName);
  let username = $derived($user?.username);
  let role = $derived($user?.role);
  let avatarUrl = $derived($user?.avatarUrl ?? "/missing.png");
  let mainSearchContainer = $state();
  let menuSearchContainer = $state();
  let menuToggled = $state(false);

  let windowWidth = $state(0);

  $effect(() => {
    if ($isLg) menuToggled = false;
  });

  $effect(() => {
    if (menuToggled === false && !$isLg) {
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
    [SeriesButton, "Series", "/series", "series"],
    [DashboardButton, "Dashboard", "/dashboard", "dashboard", true],
  ];

  let mounted = $state();
  onMount(() => {
    mounted = true;
  });

  afterNavigate(() => {
    menuToggled = false;
    searchData._term = "";
  });
</script>

<header class="fixed w-full bg-white text-dark z-100 shadow-lg">
  <div class="z-10 flex not-lg:justify-center w-cap-2 p-2 lg:p-4 gap-4">
    <div class="flex items-center gap-4">
      <div
        class="rounded-full bg-background transition-all duration-200 shadow-lg hover:brightness-102 hover:scale-102"
      >
        <a href="/" title="logo">
          <svg class="not-lg:h-10 h-20" viewBox="0 0 92.09 61.75">
            <g fill="none" stroke-linecap="round" stroke-linejoin="round">
              <g stroke="#fff" stroke-width="9px">
                <path
                  d="M27.7,24.17c-.56-.68-3.14-3.77-6.92-3.49-7.25.55-9.22,6.21-9.17,10.75.07,5.26,5.65,9.58,11.08,9.61,24.87.15,31.49-26.63,31.49-26.63l.45,26.29c2.71.86,16.46-26.49,16.46-26.49l.2,26.48"
                /><path
                  d="M87.59,7.61c-.51-.77-2.06-3.08-4.81-3.11-2.58-.03-5.27,2.82-5.27,4.81l-.12,47.94"
                /><path d="M4.5,47.39h83.05" />
              </g>
              <g stroke="#495d83" stroke-width="4px">
                {#if mounted}
                  <path
                    in:draw={{
                      duration: 700,
                      easing: (t) => {
                        const mid = 0.48;
                        if (t < mid) {
                          return t;
                        } else {
                          const nT = (t - mid) / (1 - mid);
                          const qP = nT * nT * nT;
                          return mid + qP * (1 - mid);
                        }
                      },
                    }}
                    d="M27.67,23.97c-.56-.68-3.14-3.77-6.92-3.49-7.25.55-9.22,6.21-9.17,10.75.07,5.26,5.65,9.58,11.08,9.61,24.87.15,31.49-26.63,31.49-26.63l.45,26.29c2.71.86,16.46-26.49,16.46-26.49l.2,26.48"
                  />
                {/if}
                <path
                  d="M87.57,7.41c-.51-.77-2.06-3.08-4.81-3.11-2.58-.03-5.27,2.82-5.27,4.81l-.12,47.94"
                />
                <path d="M4.48,47.19h83.05" />
              </g>
            </g>
          </svg>
        </a>
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
            <button onclick={handleLogout}
              ><span>Sign Out{" "}</span><svg
                class="inline fill-white h-6"
                viewBox="0 0 24 24"
              >
                <path
                  fill-rule="evenodd"
                  clip-rule="evenodd"
                  d="M7 3C6.44772 3 6 3.44772 6 4C6 4.55228 6.44772 5 7 5H18C18.5523 5 19 5.44772 19 6V18C19 18.5523 18.5523 19 18 19H7C6.44772 19 6 19.4477 6 20C6 20.5523 6.44772 21 7 21H18C19.6569 21 21 19.6569 21 18V6C21 4.34315 19.6569 3 18 3H7ZM12.7071 7.29289C12.3166 6.90237 11.6834 6.90237 11.2929 7.29289C10.9024 7.68342 10.9024 8.31658 11.2929 8.70711L13.5858 11H4C3.44772 11 3 11.4477 3 12C3 12.5523 3.44772 13 4 13H13.5858L11.2929 15.2929C10.9024 15.6834 10.9024 16.3166 11.2929 16.7071C11.6834 17.0976 12.3166 17.0976 12.7071 16.7071L16.7071 12.7071C17.0976 12.3166 17.0976 11.6834 16.7071 11.2929L12.7071 7.29289Z"
                >
                </path>
              </svg></button
            >
          </div>
        {:else}
          <span>wanna</span>
          <div class="duo-btn duo-primary">
            <a class="no-underline!" href="/login">
              <span>Log In{" "}</span>
              <svg class="inline fill-white h-6" viewBox="0 0 24 24">
                <path
                  d="M7 3C6.44772 3 6 3.44772 6 4C6 4.55228 6.44772 5 7 5H18C18.5523 5 19 5.44772 19 6V18C19 18.5523 18.5523 19 18 19H7C6.44772 19 6 19.4477 6 20C6 20.5523 6.44772 21 7 21H18C19.6569 21 21 19.6569 21 18V6C21 4.34315 19.6569 3 18 3H7ZM12.7071 7.29289C12.3166 6.90237 11.6834 6.90237 11.2929 7.29289C10.9024 7.68342 10.9024 8.31658 11.2929 8.70711L13.5858 11H4C3.44772 11 3 11.4477 3 12C3 12.5523 3.44772 13 4 13H13.5858L11.2929 15.2929C10.9024 15.6834 10.9024 16.3166 11.2929 16.7071C11.6834 17.0976 12.3166 17.0976 12.7071 16.7071L16.7071 12.7071C17.0976 12.3166 17.0976 11.6834 16.7071 11.2929L12.7071 7.29289Z"
                >
                </path>
              </svg>
            </a>
          </div>
          <span class="text-dark">or</span>
          <div class="duo-btn duo-primary">
            <a class="no-underline!" href="/login?register=true">
              <svg class="inline fill-white h-6" viewBox="0 0 24 24">
                <path
                  fill-rule="evenodd"
                  clip-rule="evenodd"
                  d="M16 6C14.3432 6 13 7.34315 13 9C13 10.6569 14.3432 12 16 12C17.6569 12 19 10.6569 19 9C19 7.34315 17.6569 6 16 6ZM11 9C11 6.23858 13.2386 4 16 4C18.7614 4 21 6.23858 21 9C21 10.3193 20.489 11.5193 19.6542 12.4128C21.4951 13.0124 22.9176 14.1993 23.8264 15.5329C24.1374 15.9893 24.0195 16.6114 23.5631 16.9224C23.1068 17.2334 22.4846 17.1155 22.1736 16.6591C21.1979 15.2273 19.4178 14 17 14C13.166 14 11 17.0742 11 19C11 19.5523 10.5523 20 10 20C9.44773 20 9.00001 19.5523 9.00001 19C9.00001 18.308 9.15848 17.57 9.46082 16.8425C9.38379 16.7931 9.3123 16.7323 9.24889 16.6602C8.42804 15.7262 7.15417 15 5.50001 15C3.84585 15 2.57199 15.7262 1.75114 16.6602C1.38655 17.075 0.754692 17.1157 0.339855 16.7511C-0.0749807 16.3865 -0.115709 15.7547 0.248886 15.3398C0.809035 14.7025 1.51784 14.1364 2.35725 13.7207C1.51989 12.9035 1.00001 11.7625 1.00001 10.5C1.00001 8.01472 3.01473 6 5.50001 6C7.98529 6 10 8.01472 10 10.5C10 11.7625 9.48013 12.9035 8.64278 13.7207C9.36518 14.0785 9.99085 14.5476 10.5083 15.0777C11.152 14.2659 11.9886 13.5382 12.9922 12.9945C11.7822 12.0819 11 10.6323 11 9ZM3.00001 10.5C3.00001 9.11929 4.1193 8 5.50001 8C6.88072 8 8.00001 9.11929 8.00001 10.5C8.00001 11.8807 6.88072 13 5.50001 13C4.1193 13 3.00001 11.8807 3.00001 10.5Z"
                ></path>
              </svg>
              <span>{" "}Sign Up?</span>
            </a>
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
  <PBody visible={menuToggled}>
    <button
      class="full cursor-default!"
      onclick={() => (menuToggled = false)}
      title="close-menu-overlay"
    ></button>
  </PBody>
  {#if menuToggled}
    <div
      in:fly={{ y: -10, duration: 100 }}
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
                <span>log in</span>
                <svg class="inline fill-white h-5" viewBox="0 0 24 24">
                  <path
                    d="M7 3C6.44772 3 6 3.44772 6 4C6 4.55228 6.44772 5 7 5H18C18.5523 5 19 5.44772 19 6V18C19 18.5523 18.5523 19 18 19H7C6.44772 19 6 19.4477 6 20C6 20.5523 6.44772 21 7 21H18C19.6569 21 21 19.6569 21 18V6C21 4.34315 19.6569 3 18 3H7ZM12.7071 7.29289C12.3166 6.90237 11.6834 6.90237 11.2929 7.29289C10.9024 7.68342 10.9024 8.31658 11.2929 8.70711L13.5858 11H4C3.44772 11 3 11.4477 3 12C3 12.5523 3.44772 13 4 13H13.5858L11.2929 15.2929C10.9024 15.6834 10.9024 16.3166 11.2929 16.7071C11.6834 17.0976 12.3166 17.0976 12.7071 16.7071L16.7071 12.7071C17.0976 12.3166 17.0976 11.6834 16.7071 11.2929L12.7071 7.29289Z"
                  >
                  </path>
                </svg>
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
                <svg class="inline fill-white h-5" viewBox="0 0 24 24">
                  <path
                    fill-rule="evenodd"
                    clip-rule="evenodd"
                    d="M16 6C14.3432 6 13 7.34315 13 9C13 10.6569 14.3432 12 16 12C17.6569 12 19 10.6569 19 9C19 7.34315 17.6569 6 16 6ZM11 9C11 6.23858 13.2386 4 16 4C18.7614 4 21 6.23858 21 9C21 10.3193 20.489 11.5193 19.6542 12.4128C21.4951 13.0124 22.9176 14.1993 23.8264 15.5329C24.1374 15.9893 24.0195 16.6114 23.5631 16.9224C23.1068 17.2334 22.4846 17.1155 22.1736 16.6591C21.1979 15.2273 19.4178 14 17 14C13.166 14 11 17.0742 11 19C11 19.5523 10.5523 20 10 20C9.44773 20 9.00001 19.5523 9.00001 19C9.00001 18.308 9.15848 17.57 9.46082 16.8425C9.38379 16.7931 9.3123 16.7323 9.24889 16.6602C8.42804 15.7262 7.15417 15 5.50001 15C3.84585 15 2.57199 15.7262 1.75114 16.6602C1.38655 17.075 0.754692 17.1157 0.339855 16.7511C-0.0749807 16.3865 -0.115709 15.7547 0.248886 15.3398C0.809035 14.7025 1.51784 14.1364 2.35725 13.7207C1.51989 12.9035 1.00001 11.7625 1.00001 10.5C1.00001 8.01472 3.01473 6 5.50001 6C7.98529 6 10 8.01472 10 10.5C10 11.7625 9.48013 12.9035 8.64278 13.7207C9.36518 14.0785 9.99085 14.5476 10.5083 15.0777C11.152 14.2659 11.9886 13.5382 12.9922 12.9945C11.7822 12.0819 11 10.6323 11 9ZM3.00001 10.5C3.00001 9.11929 4.1193 8 5.50001 8C6.88072 8 8.00001 9.11929 8.00001 10.5C8.00001 11.8807 6.88072 13 5.50001 13C4.1193 13 3.00001 11.8807 3.00001 10.5Z"
                  ></path>
                </svg>
                <span>sign up?</span>
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
          <ul class="space-y-2 bg-white p-2 rounded-xl" id="side-bar">
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
