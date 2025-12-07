<script>
    import Post from "$lib/components/post/Post.svelte";

    let { data, slug } = $props();
    let { author_slug, author_name, title, content, published_at, tags } = data;

    let date = $derived.by(() => {
        const dt = new Date(published_at.split(" ")[0]);
        const options = { year: "numeric", month: "short", day: "numeric" };
        return dt.toLocaleDateString("en-US", options);
    });
</script>

<section class="flex not-xl:flex-col">
    <div
        class="flex grow flex-col bg-white/90 p-4 gap-4 rounded-xl not-xl:rounded-b-none xl:rounded-tr-none"
    >
        <div class="space-y-2 text-base">
            <h1 class="text-4xl">{title}</h1>
            <div class="inline gap-2 text-dark/60">
                <a
                    class="font-semibold text-dark/80 text-nowrap"
                    href={`/profiles/${author_slug}`}>{author_name}</a
                >
                {#if tags?.length > 0}
                    <span class="font-bold select-none">&middot;</span>
                    <ul class="inline text-dark *:inline space-x-1">
                        {#each tags as tag}
                            <li
                                class="rounded-full px-1 border-2 border-primary *:no-underline! has-hover:bg-primary duration-100 transition-colors"
                            >
                                <a
                                    class="text-primary hover:text-white/90 hover:*:text-white/90 duration-100 transition-colors"
                                    href={`/tags/${tag}`}
                                    ><span class="text-primary/60">#</span
                                    >{tag}</a
                                >
                            </li>
                        {/each}
                    </ul>
                {/if}
            </div>
            <div class="flex items-center gap-2 py-4">
                <hr class="grow border" />
                <span class="text-nowrap">{date}</span>
                <hr class="xl:hidden grow border" />
            </div>
        </div>
        <Post {content} />
    </div>
    <div class="min-w-60">
        <div
            class="w-full *:full bg-white/90 h-40 rounded-xl not-xl:rounded-t-none xl:rounded-l-none"
        >
            <div class="xl:hidden p-4">
                <hr class="border" />
            </div>
            <div class="p-4">Hello?</div>
        </div>
        <svg class="not-xl:hidden w-6 fill-white/90" viewBox="0 0 24 24">
            <path d="M 0,0 L 12,0 A 12,12 0 0 0 0,12 Z" />
        </svg>
    </div>
</section>
