<script>
    import { autoHResize } from "$lib/client/auto-resize";
    import { auth, user } from "$lib/client/user";
    import { textToDate } from "$lib/common";
    import Post from "$lib/components/post/Post.svelte";
    import { onMount } from "svelte";
    import { gsap } from "gsap";
    import Heart from "$lib/components/svgs/Heart.svelte";
    import Diamond from "$lib/components/svgs/Diamond.svelte";
    import Club from "$lib/components/svgs/Club.svelte";
    import Spade from "$lib/components/svgs/Spade.svelte";
    import { fade } from "svelte/transition";

    let { data, slug } = $props();

    let {
        id,
        author_slug,
        author_name,
        author_avatar_url,
        title,
        content,
        published_at,
        tags,
    } = data;

    let userAvatarUrl = $derived($user?.avatarUrl ?? "/missing.png");
    let authorAvatarUrl = $derived(author_avatar_url ?? "/missing.png");

    let comments = $state({
        current: "",
        initialized: false,
        fetching: false,
        sending: false,
        endReached: false,
        lastId: 0,
        data: [],
    });

    let date = $derived(textToDate(published_at));

    const limit = 3;
    let discussionSection = $state();

    const updateComments = (newComments) => {
        comments.data = [...comments.data, ...newComments].sort(
            (a, b) => b.id - a.id,
        );

        const length = comments.data.length;

        if (length > 0) {
            comments.lastId = comments.data[length - 1].id;
        }

        if (newComments < limit) {
            comments.endReached = true;
        }
    };

    const fetchComments = async () => {
        if (comments.fetching) return;
        comments.fetching = true;
        const api =
            comments.lastId === 0
                ? `/api/posts/id/${id}/comments?limit=${limit}`
                : `/api/posts/id/${id}/comments?limit=${limit}&before=${comments.lastId}`;
        const res = await fetch(api);
        if (res.ok) {
            const data = await res.json();
            updateComments(data.comments);
        }
        comments.fetching = false;
    };

    onMount(() => {
        const onScrolled = gsap.to(discussionSection, {
            scrollTrigger: {
                trigger: discussionSection,
                once: true,
                start: "bottom bottom",
                onEnter: fetchComments,
            },
        });

        const triggerInstance = onScrolled.scrollTrigger;

        return () => {
            triggerInstance?.kill();
            onScrolled?.kill();
        };
    });
</script>

{#snippet comment_snippet(
    content,
    username,
    displayName,
    date,
    avatarUrl,
    userRole,
)}
    {@const someone = username !== undefined && displayName}
    <li transition:fade={{ duration: 200 }} class="flex py-4">
        <div
            class="ml-2 min-w-10 lg:min-w-12 w-10 lg:w-12 h-10 lg:h-12 rounded-full shadow-md overflow-hidden"
        >
            {#if someone}
                <a class="full" href={"/profiles/" + username}>
                    <img
                        class="full object-cover"
                        src={avatarUrl ?? "/missing.png"}
                        alt="comment-posting-avatar"
                    />
                </a>
            {:else}
                <img
                    class="full object-cover"
                    src={avatarUrl ?? "/missing.png"}
                    alt="comment-posting-avatar"
                />
            {/if}
        </div>
        <div class="relative flex flex-col grow">
            <div class="pl-2 -translate-y-2">
                <div class="relative w-fit">
                    <div
                        class="w-fit p-2 bg-primary/20 rounded-2xl rounded-tl-md"
                    >
                        <div class="flex items-center lg:text-base">
                            {#if someone}
                                <a
                                    class="font-semibold"
                                    href={"/profiles/" + username}
                                    >{displayName}
                                </a>
                            {:else}
                                <span class="font-normal italic">Anonymous</span
                                >
                            {/if}
                            <span
                                class="*:w-8 hover:*:translate-x-1 *:transition-all *:duration-200 tooltip-container"
                                data-tooltip={userRole === "admin"
                                    ? "he's THE admin!"
                                    : userRole === "moderator"
                                      ? "a mod!"
                                      : userRole === "user"
                                        ? "user"
                                        : "?"}
                            >
                                {#if userRole === "admin"}
                                    <Heart class="fill-accent-red" />
                                {:else if userRole === "moderator"}
                                    <Diamond class="fill-accent-red" />
                                {:else if userRole === "user"}
                                    <Club class="fill-dark" />
                                {:else}
                                    <Spade class="fill-dark" />
                                {/if}
                            </span>
                        </div>

                        <p class="w-fit min-h-6 whitespace-pre">
                            {content}
                        </p>
                    </div>
                    <div
                        class="absolute flex min-w-20 w-full justify-between gap-2 left-0 top-full text-sm"
                    >
                        <span class="pl-2">{date ?? "new"}</span>
                        <div class="text-nowrap *:cursor-pointer">
                            <span>reply</span>
                            <span>report</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </li>
{/snippet}

<section class="flex flex-col gap-4 pb-4 *:drop-shadow-xl">
    <div class="flex not-xl:flex-col">
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
                                src={authorAvatarUrl}
                                alt="author-avatar"
                            />
                        </div>
                        <div class="flex flex-col">
                            <a
                                class="font-semibold text-dark/80 text-nowrap"
                                href={`/profiles/${author_slug}`}
                                >{author_name}
                            </a>
                            <span>{author_slug}</span>
                        </div>
                    </div>
                </div>
            </div>
            <svg class="not-xl:hidden w-6 fill-white/90" viewBox="0 0 24 24">
                <path d="M 0,0 L 12,0 A 12,12 0 0 0 0,12 Z" />
            </svg>
        </div>
    </div>

    <div class="w-full xl:w-[calc(100%-15rem)] bg-white/90 p-4 rounded-xl">
        <h4 class="text-lg lg:text-2xl">Join the discussion!</h4>
        <div class="flex flex-col gap-4" bind:this={discussionSection}>
            <hr class="border-t-3 border-dark mb-6" />
            <div class="flex gap-8">
                <div
                    class="w-12 lg:w-20 h-12 lg:h-20 border-dark border-2 rounded-full overflow-hidden"
                >
                    <img
                        class="full object-cover"
                        src={userAvatarUrl}
                        alt="comment-posting-avatar"
                    />
                </div>
                <div class="grow flex flex-col gap-4 relative">
                    <svg
                        class="absolute fill-primary/60 top-6 lg:top-10 -left-4 -translate-y-1/2 w-4 h-4"
                        viewBox="0 0 24 24"
                    >
                        <polygon points="0,12 24,0 24,24" /></svg
                    >
                    <textarea
                        name="comment-input"
                        class="text-base w-full min-h-10 lg:min-h-20 overflow-hidden resize-none outline-dark bg-primary/20 outline-2 p-2 rounded-xl"
                        use:autoHResize
                        bind:value={comments.current}
                    ></textarea>
                    <div class="ml-auto w-fit duo-btn duo-blue">
                        <button
                            class="fill-white"
                            type="button"
                            disabled={comments.sending ||
                                comments.current.length < 1}
                            onclick={async () => {
                                const headers =
                                    $user !== undefined
                                        ? {
                                              Authorization: auth(),
                                              "Content-Type":
                                                  "application/json",
                                          }
                                        : {
                                              "Content-Type":
                                                  "application/json",
                                          };

                                comments.sending = true;

                                const res = await fetch(
                                    `/api/posts/id/${id}/comments/new`,
                                    {
                                        method: "PUT",
                                        headers,
                                        body: JSON.stringify({
                                            content: comments.current,
                                        }),
                                    },
                                );
                                if (res.ok) {
                                    const data = await res.json();
                                    const userData =
                                        $user !== undefined
                                            ? {
                                                  display_name:
                                                      $user.displayName,
                                                  username: $user.username,
                                              }
                                            : {};
                                    const newComment = {
                                        id: data.comment_id,
                                        avatar_url: userAvatarUrl,
                                        content: comments.current,
                                        created_at: undefined,
                                        ...userData,
                                    };
                                    comments.current = "";
                                    updateComments([newComment]);
                                }
                                comments.sending = false;
                            }}
                        >
                            <svg
                                class="w-6 -scale-x-100 -translate-y-0.5 inline-block"
                                viewbox="0 0 24 24"
                            >
                                <g>
                                    <path
                                        fill-rule="evenodd"
                                        clip-rule="evenodd"
                                        d="M3.3572 3.23397C3.66645 2.97447 4.1014 2.92638 4.45988 3.11204L20.7851 11.567C21.1426 11.7522 21.3542 12.1337 21.322 12.5351C21.2898 12.9364 21.02 13.2793 20.6375 13.405L13.7827 15.6586L10.373 22.0179C10.1828 22.3728 9.79826 22.5789 9.39743 22.541C8.9966 22.503 8.65762 22.2284 8.53735 21.8441L3.04564 4.29872C2.92505 3.91345 3.04794 3.49346 3.3572 3.23397ZM5.67123 5.99173L9.73507 18.9752L12.2091 14.361C12.3304 14.1347 12.5341 13.9637 12.7781 13.8835L17.7518 12.2484L5.67123 5.99173Z"
                                    >
                                    </path>
                                </g>
                            </svg>
                            Send
                        </button>
                    </div>
                </div>
            </div>
        </div>
        <ul>
            {#each comments.data as { id, avatar_url, content, created_at, display_name, username, user_role }, index (id)}
                {@render comment_snippet(
                    content,
                    username,
                    display_name,
                    created_at,
                    avatar_url,
                    user_role,
                )}
            {/each}
        </ul>
        <div class="mt-8 mx-auto w-fit duo-btn duo-blue">
            <button
                disabled={comments.endReached || comments.fetching}
                onclick={fetchComments}
                >{comments.endReached
                    ? "No more to read ૮´•˕•`ა"
                    : "Read more"}</button
            >
        </div>
    </div>
</section>
