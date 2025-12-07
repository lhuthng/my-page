<script>
    import { goto } from "$app/navigation";
    import { page } from "$app/stores";
    import { logout, user } from "$lib/client/user";
    import SearchButton from "./buttons/SearchButton.svelte";

    let displayName = $derived($user?.displayName);
    let username = $derived($user?.username);
    let role = $derived($user?.role);

    const handleLogout = async () => {
        if ($page.url.pathname.startsWith("/dashboard")) {
            goto("/");
        }
        logout();
    };
</script>

<header class="fixed w-full bg-white text-dark shadow-lg z-100">
    <div class="flex not-lg:justify-center w-cap-2 p-4 gap-4">
        <div class="flex items-center gap-4">
            <div
                class="rounded-full bg-background transition-all duration-200 shadow-lg hover:brightness-102 hover:scale-102"
            >
                <a href="/"
                    ><img
                        class="not-lg:h-14"
                        src={"/logo.png"}
                        alt="logo icon"
                    /></a
                >
            </div>
        </div>
        <div class="grow not-lg:hidden">
            <a
                class="font-semibold text-3xl text-dark transition-all duration-200 hover:scale-102"
                href="/">Huu Thang's blog</a
            >
            <div
                class="flex max-w-160 h-10 items-center rounded-xl border-2 transition-colors duration-50 bg-dark has-active:bg-primary border-dark has-active:border-primary overflow-hidden"
            >
                <div class="h-full grow grid px-2 bg-white">
                    <input class="w-full" placeholder="Search" />
                </div>
                <div class="flex items-center h-full">
                    <div class="h-full px-2 border-l-2 rounded-r-lg bg-white">
                        <select class="full text-dark" name="search-options">
                            <option>Name</option>
                            <option>Tag</option>
                            <option>Category</option>
                            <option>User</option>
                        </select>
                    </div>

                    <div>
                        <SearchButton
                            class="p-2 w-10 h-full transition-transform duration-50 active:translate-y-0.5"
                            fill="white"
                        />
                    </div>
                </div>
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
                        <a class="no-underline!" href="/login"> Sign Up? </a>
                    </div>
                {/if}
            </div>
            {#if displayName && username}
                <div>
                    Welcome back, <a
                        class="font-bold"
                        href="/profiles/{username}">{displayName}</a
                    >
                </div>
            {/if}
        </div>
    </div>
</header>

<style lang="postcss">
    @reference "../../app.css";
</style>
