<script>
    import { goto } from "$app/navigation";
    import { page } from "$app/stores";
    import { logout, user } from "$lib/client/user";
    import { useDebounce } from "$lib/effects/debounce";
    import { fly } from "svelte/transition";
    import SearchButton from "./buttons/SearchButton.svelte";

    let displayName = $derived($user?.displayName);
    let username = $derived($user?.username);
    let role = $derived($user?.role);
    let avatarUrl = $derived($user?.avatarUrl ?? "/missing.png");

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
            const res = await fetch(
                `/api/users?term=${encodeURI(term)}&size=5`,
                {
                    method: "GET",
                },
            );

            if (res.ok) {
                searchData.result = await res.json();
            }
        } else if (type === "Tags") {
            searchData.result = undefined;
        } else if (type === "Post") {
            const res = await fetch(
                `/api/posts?term=${encodeURI(term)}&size=5`,
                {
                    method: "GET",
                },
            );

            if (res.ok) {
                searchData.result = await res.json();
            }
        }
        searchData.status = "fetched";
    };
</script>

<header class="fixed w-full bg-white text-dark shadow-lg z-100">
    <div class="flex not-lg:justify-center w-cap-2 p-2 lg:p-4 gap-4">
        <div class="flex items-center gap-4">
            <div
                class="rounded-full bg-background transition-all duration-200 shadow-lg hover:brightness-102 hover:scale-102"
            >
                <a href="/"
                    ><img
                        class="not-lg:h-10 h-20"
                        src={"/logo.svg"}
                        alt="logo icon"
                    /></a
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
                            onclick={() =>
                                search(searchData.selection, searchData._term)}
                        />
                    </div>
                </div>
            </div>
            <div
                class="absolute z-9 left-0 w-full top-full h-fit drop-shadow-sm"
            >
                {#if searchData.status !== "init"}
                    <div
                        in:fly={{ y: -10, duration: 200 }}
                        out:fly={{ y: 10, duration: 200 }}
                        class="w-full h-fit bg-white rounded-b-xl p-4"
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
                                                    src={avatarUrl ??
                                                        "/missing.png"}
                                                    alt="mini-avatar"
                                                />
                                                <a
                                                    href={"/profiles/" +
                                                        username}
                                                    >{displayName}</a
                                                >
                                            </li>
                                        {/each}
                                    </ul>
                                {/if}
                            {:else if searchData.selection === "Tag"}
                                <span
                                    >Tag searching is not supported (yet) <span
                                        class="text-nowrap font-bold"
                                        >(づ_ど)</span
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
                                                    src={coverUrl ??
                                                        "/missing.png"}
                                                    alt="mini-avatar"
                                                />
                                                <a href={"posts/" + slug}
                                                    >{title}</a
                                                >
                                            </li>
                                        {/each}
                                    </ul>
                                {/if}
                            {/if}
                        {/if}
                    </div>
                {/if}
            </div>
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
                        <a class="no-underline!" href="/login?register=true">
                            Sign Up?
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
</header>

<style lang="postcss">
    @reference "../../app.css";
</style>
