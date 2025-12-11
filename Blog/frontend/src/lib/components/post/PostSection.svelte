<script>
    import Post from "./Post.svelte";

    let { title, tags, date, content, author } = $props();
</script>

<section class="flex not-xl:flex-col">
    <div
        class="flex grow flex-col bg-white/90 p-4 gap-4 rounded-xl not-xl:rounded-b-none xl:rounded-tr-none"
    >
        <div class="space-y-2 text-base">
            <h1 class="text-2xl lg:text-4xl">{title}</h1>
            <div class="inline gap-2 text-dark/60">
                {#if tags?.length > 0}
                    <ul class="inline text-dark *:inline space-x-1">
                        {#each tags as tag}
                            <li
                                class="rounded-full px-1 border-2 border-primary *:no-underline! has-hover:bg-primary duration-100 transition-colors"
                            >
                                <a
                                    class="text-primary hover:text-white/90 hover:*:text-white/90 duration-100 transition-colors"
                                    href={`/tags/${tag}`}
                                >
                                    <span class="text-gray-300">#</span>
                                    {tag}
                                </a>
                            </li>
                        {/each}
                    </ul>
                {/if}
            </div>
            <div class="flex items-center gap-2 py-4">
                <hr class="xl:hidden grow border" />
                <span class="text-nowrap">{date}</span>
                <hr class="grow border" />
            </div>
        </div>
        <Post {content} />
    </div>
    <div class="min-w-60">
        <div
            class="w-full *:full bg-white/90 rounded-xl not-xl:rounded-t-none xl:rounded-l-none"
        >
            <div class="xl:hidden p-4">
                <hr class="border" />
            </div>
            <div class="pl-4 pt-4">Written by:</div>
            <div class="flex flex-col gap-2 p-4 pt-2 text-dark">
                <div
                    class="flex items-center gap-2 bg-secondary/60 p-2 rounded-lg"
                >
                    <div
                        class="w-fit h-fit bg-radial from-white to-secondary rounded-full overflow-hidden"
                    >
                        <img
                            class="min-w-16 w-16 h-16 object-contain"
                            src={author.avatarUrl ?? "/missing.png"}
                            alt="author-avatar"
                        />
                    </div>
                    <div class="flex flex-col">
                        <a
                            class="font-semibold text-dark/80 text-nowrap"
                            href={`/profiles/${author.username}`}
                            >{author.displayName}
                        </a>
                        <span>{author.username}</span>
                    </div>
                </div>
            </div>
        </div>
        <svg class="not-xl:hidden w-6 fill-white/90" viewBox="0 0 24 24">
            <path d="M 0,0 L 12,0 A 12,12 0 0 0 0,12 Z" />
        </svg>
    </div>
</section>
