<script>
    import { goto } from "$app/navigation";
    import { logout, user } from "$lib/client/user";

    let displayName = $derived($user?.displayName);
    let username = $derived($user?.username);
    let role = $derived($user?.role);

    $effect(() => {
        console.log($user);
    });
</script>

<header class="fixed w-full h-18 bg-gray-200 z-10">
    <div class="flex w-full justify-between p-4">
        <div class="bg-red-500 rounded-full w-10 h-10"></div>
        <div class="flex flex-col items-end">
            <div class="flex gap-8">
                <span>Tags</span>
                <span>Archives</span>
                <span>Portfolio</span>
                <span>About</span>
            </div>
            <div class="flex gap-4">
                {#if displayName && username}
                    <span
                        >Hi <a class="text-blue-700" href={`/users/${username}`}
                            >{displayName}</a
                        ></span
                    >
                    {#if role === "admin"}
                        <a
                            class="ring-1 rounded-full px-2"
                            href="/dashboard/media/manager">Management</a
                        >
                    {/if}
                    <button
                        class="ring-1 rounded-full px-2"
                        type="button"
                        onclick={() => logout()}>Log out</button
                    >
                {:else}
                    <span>Sign up</span>
                    <a
                        class="text-white rounded-full bg-black px-2"
                        href="/login">Log in</a
                    >
                {/if}
            </div>
        </div>
    </div>
</header>
