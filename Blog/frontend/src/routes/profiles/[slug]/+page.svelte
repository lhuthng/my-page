<script>
    import { auth, user } from "$lib/client/user.js";
    import PostCard from "$lib/components/home/PostCard.svelte";
    import Club from "$lib/components/svgs/Club.svelte";
    import Diamond from "$lib/components/svgs/Diamond.svelte";
    import Heart from "$lib/components/svgs/Heart.svelte";
    import Spade from "$lib/components/svgs/Spade.svelte";
    import { onMount } from "svelte";
    import { flip } from "svelte/animate";
    import { fade, fly } from "svelte/transition";
    import { autoHResize } from "$lib/client/auto-resize.js";

    const { data } = $props();

    let response = $derived(data.response);
    let username = $derived(response.username);
    let role = $derived(response.role);
    let displayName = $state(data.response.display_name);
    let bio = $state(data.response.bio);
    let postContainer = $state();

    let editor = $state({
        isEditing: false,
        isFetching: false,
        displayName: "",
        bio: "",
    });

    const hasPosts = $derived(role === "moderator" || role === "admin");

    let posts = $state({
        status: "initial",
        fetchingMore: false,
        fetchedAll: false,
        message: "",
        data: [],
    });

    const limit = 3;

    $effect(async () => {
        if (!hasPosts) {
            return;
        }

        posts.status = "fetching";

        const res = await fetch(
            `/api/users/${username}/posts?limit=${limit}&offset=0`,
        );

        if (!res.ok) {
            posts.status = "failed";
            posts.message = await res.text();
        } else {
            posts.status = "fetched";
            const data = (await res.json()).posts;
            if (data.length < limit) {
                posts.fetchedAll = true;
            }
            posts.data = data;
        }
    });

    const fetchMore = async () => {
        if (posts.fetchedAll || posts.data.fetchingMore) return;

        const before = posts.data.length;
        posts.data.fetchingMore = true;

        const res = await fetch(
            `/api/users/${username}/posts?limit=${limit}&offset=${before}`,
        );

        if (!res.ok) {
            posts.status = "failed";
            posts.message = await res.text();
        } else {
            const data = (await res.json()).posts;

            if (data.length < limit) {
                posts.fetchedAll = true;
            }

            posts.data = [...posts.data, ...data];
            posts.fetchingMore = false;
        }
    };
</script>

<section
    class="flex flex-col gap-4 *:bg-white/90 *:rounded-xl *:p-4 pb-8"
    key={username}
>
    <div class="flex gap-4">
        <div class="space-y-4 min-w-60">
            <img
                class="w-60 h-60 rounded-bl-xl rounded-tr-xl"
                src="/missing.png"
                alt="avatar"
            />
            <div class="grid grid-cols-2 gap-2">
                <div class="duo-btn duo-blue">
                    <button disabled>Message</button>
                </div>
                <div class="duo-btn duo-green">
                    <button disabled>Follow</button>
                </div>
            </div>
        </div>
        <div class="w-full">
            <div class="flex items-end">
                {#if editor.isEditing}
                    <input
                        class="font-bold text-3xl bg-accent-green/20 field-sizing-content"
                        type="text"
                        name="username"
                        autocomplete="off"
                        bind:value={editor.displayName}
                    />
                {:else}
                    <h2 class="inline font-bold text-3xl">{displayName}</h2>
                {/if}
                <span
                    class="*:w-10 hover:*:translate-x-1 *:transition-all *:duration-200 tooltip-container"
                    data-tooltip={role}
                >
                    {#if role === "admin"}
                        <Heart class="fill-accent-red" />
                    {:else if role === "moderator"}
                        <Diamond class="fill-accent-red" />
                    {:else if role === "user"}
                        <Club class="fill-dark" />
                    {:else}
                        <Spade class="fill-dark" />
                    {/if}
                </span>
                {#if $user?.username === username}
                    <span class="grow"></span>
                    <div
                        class="duo-btn col-span-full"
                        class:duo-green={!editor.isEditing}
                        class:duo-red={editor.isEditing}
                    >
                        <button
                            onclick={() => {
                                const isEditing = !editor.isEditing;
                                if (isEditing) {
                                    editor.bio = bio;
                                    editor.displayName = displayName;
                                }
                                editor.isEditing = isEditing;
                            }}
                            disabled={editor.isFetching}
                            >{editor.isEditing ? "Cancel" : "Edit"}</button
                        >
                    </div>
                {/if}
            </div>
            <span class="italic text-primary/80 *:select-none"
                ><span>./</span>{username}<span>/</span></span
            >
            {#if editor.isEditing}
                <div class="py-4 bg-accent-green/20 rounded-lg">
                    <textarea
                        name="bio"
                        class="text-lg w-full overflow-hidden resize-none outline-none"
                        bind:value={editor.bio}
                        use:autoHResize
                    ></textarea>
                    <div class="flex justify-end">
                        <div class="duo-btn duo-green w-fit">
                            <button
                                onclick={async () => {
                                    editor.isFetching = true;
                                    const res = await fetch("/api/users/me", {
                                        method: "PATCH",
                                        headers: {
                                            Authorization: auth(),
                                            "Content-Type": "application/json",
                                        },
                                        body: JSON.stringify({
                                            display_name: editor.displayName,
                                            bio: editor.bio,
                                        }),
                                    });
                                    editor.isFetching = false;
                                    editor.isEditing = false;
                                    if (res.ok) {
                                        displayName = editor.displayName;
                                        bio = editor.bio;
                                        const data = [...posts.data];
                                        data.forEach(
                                            (post) =>
                                                (post.author_name =
                                                    displayName),
                                        );
                                        posts.data = [...data];
                                    }
                                }}
                                disabled={editor.isFetching}>Submit</button
                            >
                        </div>
                    </div>
                </div>
            {:else}
                <p class="py-4 text-lg">{bio}</p>
            {/if}
        </div>
    </div>
    {#if hasPosts}
        <div class="flex flex-col gap-4">
            <h2 class="inline font-bold text-2xl">Posts</h2>
            {#if posts.status === "fetching"}
                <div class="flex justify-center items-center p-4 text-xl">
                    Loading...
                </div>
            {:else if posts.status === "failed"}
                <div class="flex justify-center items-center p-4 text-xl">
                    Failed to load the posts, try refreshing the pages!
                </div>
            {:else if posts.status === "fetched"}
                <ul
                    bind:this={postContainer}
                    class="grid grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4"
                >
                    {#each posts.data as { title, slug, excerpt, author_name, author_slug, tag_slugs, url }, index (slug)}
                        <li
                            in:fly={{ y: -20, duration: 500 }}
                            out:fade={{ duration: 150 }}
                        >
                            <PostCard
                                {title}
                                {slug}
                                {excerpt}
                                author={{
                                    name: author_name,
                                    slug: author_slug,
                                }}
                                tags={tag_slugs}
                                src={url}
                            />
                        </li>
                    {/each}
                </ul>
                <div
                    class="flex col-span-full full items-center justify-center"
                >
                    <div class="duo-btn duo-green">
                        <button
                            disabled={posts.fetchingMore || posts.fetchedAll}
                            onclick={fetchMore}
                            >{posts.fetchingMore
                                ? "loading"
                                : posts.fetchedAll
                                  ? "no more to load"
                                  : "load more"}</button
                        >
                    </div>
                </div>
            {/if}
        </div>
    {/if}
</section>
