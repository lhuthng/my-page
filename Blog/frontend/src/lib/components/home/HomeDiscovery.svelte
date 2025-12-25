<script>
    import PostCard from "./PostCard.svelte";
    import gsap from "gsap";
    import { Flip } from "gsap/Flip";
    import { tick, untrack } from "svelte";
    import { fade, fly } from "svelte/transition";

    const { featuredPosts } = $props();

    let categories = $state(
        ["programming", "art", "music"].map((tag) => ({
            value: tag,
            selected: false,
        })),
    );

    let tabContainer = $state();
    let discoverTab = $state();
    let freshTab = $state();

    let tab = $state({
        preindex: 1,
        index: 1,
        container: null,
        discover: null,
        fresh: null,
    });

    let fresh = $state({
        status: "unset",
        cache: [],
    });

    $effect(() => {
        if (!tab.container || !tab.discover || !tab.fresh) return;

        if (tab.preindex === tab.index) return;

        tab.preindex = tab.index;

        const state = Flip.getState(tab.container);

        if (tab.index !== 1) {
            tab.discover.classList.toggle("hidden", true);
            tab.fresh.classList.toggle("hidden", false);
            tab.fresh.classList.toggle("grid", true);
        } else {
            tab.discover.classList.toggle("hidden", false);
            tab.fresh.classList.toggle("hidden", true);
            tab.fresh.classList.toggle("grid", false);
        }
        Flip.from(state, { duration: 0.5, ease: "power3.inOut" });
    });

    $effect(async () => {
        if (tab.index !== 2) return;

        const { status } = untrack(() => fresh);
        if (status === "fetched" || status === "pending") return;
        fresh.status = "pending";

        const res = await fetch("api/posts/latest?limit=5&offset=0", {
            method: "GET",
        });

        if (res.ok) {
            const state = Flip.getState(tab.container);
            fresh.cache = (await res.json()).featured_posts;
            fresh.status = "fetched";

            await tick();

            Flip.from(state, { duration: 0.5, ease: "power3.inOut" });
        } else {
            fresh.cache = [];
            fresh.status = "failed";
        }
    });
</script>

{#snippet exploreMore(link)}
    <li
        class="flex justify-center items-center full min-w-22 sm:min-w-26 min-h-22 sm:min-h-26 md:min-w-34 md:min-h-34 rounded-lg border-2 border-dashed"
    >
        <div class="duo-btn duo-blue">
            <a class="no-underline!" href={link}>explore more</a>
        </div>
    </li>
{/snippet}

<div class="space-y-4 pt-4">
    <div class="flex justify-between">
        <ul id="home-tab" class="text-xl font-medium h-8">
            <li class:left={true} class:selected={tab.index === 1}>
                <button onclick={() => (tab.index = 1)}>Discover</button>
            </li>
            <li class:right={true} class:selected={tab.index === 2}>
                <button onclick={() => (tab.index = 2)}>Fresh</button>
            </li>
        </ul>
    </div>
    <div bind:this={tab.container} class="pb-2">
        <ul
            bind:this={tab.discover}
            class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4"
        >
            {#each featuredPosts as { title, slug, excerpt, author_name, author_slug, tag_slugs, url }, index (slug)}
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
            {@render exploreMore("/posts")}
        </ul>
        <ul
            bind:this={tab.fresh}
            class="hidden grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4"
        >
            {#if fresh.status === "fetched"}
                {#each fresh.cache as { title, slug, excerpt, author_name, author_slug, tag_slugs, url }, index (slug)}
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
                {@render exploreMore("/posts")}
            {:else}
                <div class="w-full col-span-full py-10 text-center">
                    Loading
                </div>
            {/if}
        </ul>
    </div>
</div>

<style lang="postcss">
    @reference "../../../app.css";

    #home-tab {
        @apply flex gap-4;
        li {
            @apply relative;
            button {
                @apply text-dark/70 relative z-10;
            }

            &::before {
                @apply absolute z-9 content-[""] -top-1 h-[calc(100%+0.25rem)] w-0 bg-linear-to-t from-background/40 via-background/30 to-primary/0 transition-all duration-200;
            }
            &::after {
                @apply absolute z-9 content-[""] bottom-0 h-1 w-0 bg-dark transition-all duration-200;
            }
            &.left::after,
            &.left::before {
                right: -0.5rem;
            }
            &.right::after,
            &.right::before {
                left: -0.5rem;
            }
        }
        li.selected > button {
            @apply text-dark relative z-10;
        }
        li.selected::after,
        li.selected::before {
            @apply w-[calc(100%+1rem)];
        }
    }
    button {
        @apply focus:outline-none;
    }
</style>
