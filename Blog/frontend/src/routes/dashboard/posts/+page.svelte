<script>
    import { user } from "$lib/client/user";
    import PostCard from "$lib/components/home/PostCard.svelte";
    import { onMount } from "svelte";
    import { fade, fly } from "svelte/transition";

    let username = $derived($user.username);

    let posts = $state({
        status: "initial",
        fetchingMore: false,
        fetchedAll: false,
        message: "",
        data: [],
    });

    const limit = 3;

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

    onMount(async () => {
        await fetchMore();
    });
</script>

<section class="flex flex-col gap-4 *:bg-white/90 *:rounded-xl *:p-4 pb-8">
    <div class="flex flex-col gap-4">
        <ul class="grid grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4">
            {#each posts.data as { title, slug, id, excerpt, author_name, author_slug, tag_slugs, url }, index (slug)}
                <li
                    in:fly={{ y: -20, duration: 500 }}
                    out:fade={{ duration: 150 }}
                >
                    <PostCard
                        {title}
                        {id}
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
        <div class="flex col-span-full full items-center justify-center">
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
    </div>
</section>
