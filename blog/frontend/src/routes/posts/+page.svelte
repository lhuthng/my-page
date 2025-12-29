<script>
    import PostCard from "$lib/components/home/PostCard.svelte";
    import { fade, fly } from "svelte/transition";

    let { data } = $props();

    let posts = $state(
        data.status === "success" ? [...data.featured_posts] : [],
    );

    let offset = $state(data.firstOffset && 0);
</script>

<div class="bg-white/90 space-y-4 rounded-xl p-4 mb-2 lg:mb-4">
    <ul
        class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4"
    >
        {#each posts as { title, slug, excerpt, author_name, author_slug, tag_slugs, url }, index (slug)}
            <li in:fly={{ y: -20, duration: 500 }} out:fade={{ duration: 150 }}>
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
    <div class="mx-auto w-fit duo-btn duo-blue">
        <button disabled>No more post</button>
    </div>
</div>
