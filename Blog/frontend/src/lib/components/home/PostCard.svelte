<script>
    import CommentButton from "../buttons/CommentButton.svelte";

    let { src, title, slug, series, excerpt, author, tags } = $props();

    let toggled = $state(false);

    let link = $derived(`/posts/${slug}`);
</script>

<div
    class="relative flex gap-4 bg-background/40 hover:bg-background/60 transition-colors duration-50 rounded-lg overflow-hidden"
>
    <img
        class="relative z-10 w-34 h-34 object-cover rounded-lg"
        {src}
        alt="thumbnail"
    />
    <div class="relative z-10 w-full pointer-events-none overflow-hidden">
        <div
            class="card absolute w-[200%] h-full flex transition-transform duration-200 left-0"
            class:toggled
        >
            <div class="h-full w-1/2 min-w-0">
                <div class="flex flex-col full py-2 min-w-0">
                    <a class="w-fit" href={link}
                        ><h1 class="text-lg">{title}</h1></a
                    >
                    <div class="flex text-sm pr-4">
                        <span class=" pointer-events-auto"
                            >by <a
                                class="text-dark!"
                                href={`/users/${author.slug}`}>{author.name}</a
                            ></span
                        >
                        {#if series !== undefined}
                            <div class="flex grow shrink gap-2">
                                <span>;</span>
                                <span class="pointer-events-auto">
                                    <span>::{series.order}</span>
                                    from
                                    <a
                                        class="text-dark!"
                                        href={`/series/${series.slug}`}
                                        >{series.name}</a
                                    >
                                </span>
                            </div>
                        {/if}
                    </div>
                    <div class="grow shrink">
                        <ul
                            class="tag-container flex flex-wrap text-sm gap-x-1 gap-y-0.5 py-0.5 pr-2 [&>li>a]:hover:no-underline!"
                        >
                            {#if tags?.length > 0}
                                <span>tags: </span>
                            {/if}
                            {#each tags as tag}
                                <li>
                                    <a href={`/tags/${tag.replace(" ", "-")}`}
                                        >{tag}</a
                                    >
                                </li>
                            {/each}
                        </ul>
                    </div>
                    <div
                        class="flex justify-between items-center text-base pr-4"
                    >
                        <div class="flex gap-1 items-center text-sm">
                            <!-- <CommentButton
                                as="span"
                                class="block w-6 h-6 fill-dark"
                            />
                            <span class="pointer-events-auto">6</span> -->
                        </div>
                        <a class="text-right" href={link}>read</a>
                    </div>
                </div>
            </div>
            <div class="absolute left-1/2 h-full w-1/2" class:toggled>
                <div
                    class="full p-2 pointer-events-auto overscroll-contain custom-scrollbar overflow-y-scroll"
                >
                    {excerpt}
                </div>
            </div>
            <svg
                class="card-btn absolute top-0 left-1/2 h-full -translate-x-2/5 has-hover:-translate-x-1/2 has-hover:fill-primary/60 fill-primary/20 transition-all duration-200 z-9"
                class:toggled
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 32 32"
            >
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <polygon
                    class="left pointer-events-auto cursor-pointer focus:outline-none"
                    points="16,0 16,32 0,16"
                    role="button"
                    tabindex="0"
                    onclick={() => (toggled = true)}
                />
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <polygon
                    class="right pointer-events-auto cursor-pointer focus:outline-none"
                    points="16,0 16,32 32,16"
                    role="button"
                    tabindex="0"
                    onclick={() => (toggled = false)}
                />
            </svg>
        </div>
    </div>
</div>

<style lang="postcss">
    @reference "../../../app.css";

    span {
        @apply text-dark/50;
    }

    a {
        @apply pointer-events-auto hover:underline;
    }

    .tag-container {
        @apply pointer-events-none;
        li > a {
            @apply px-1.5 border rounded-full hover:bg-white/40;
        }
    }

    .card.toggled {
        @apply -translate-x-1/2;
    }

    .card-btn {
        & > polygon {
            @apply transition-opacity duration-300;
        }
        & > .left {
            @apply opacity-100;
        }
        & > .right {
            @apply opacity-0;
        }

        &.toggled {
            @apply -translate-x-3/5 has-hover:-translate-x-1/2;
            & > .left {
                @apply opacity-0;
            }
            & > .right {
                @apply opacity-100;
            }
        }
    }
</style>
