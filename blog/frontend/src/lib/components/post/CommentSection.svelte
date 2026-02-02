<script>
  import { autoHResize } from "$lib/client/auto-resize";
  import { user } from "$lib/client/user";
  import { onMount } from "svelte";
  import { gsap } from "gsap";
  import { fade, fly } from "svelte/transition";
  import Heart from "$lib/components/svgs/Heart.svelte";
  import Diamond from "$lib/components/svgs/Diamond.svelte";
  import Club from "$lib/components/svgs/Club.svelte";
  import Spade from "$lib/components/svgs/Spade.svelte";
  import MarkdownIt from "markdown-it";
  import Comment from "./Comment.svelte";
  import { codeHighlightPlugin } from "$lib/custom-rules";

  const limit = 3;
  const md = new MarkdownIt().use(codeHighlightPlugin);

  let { postId } = $props();
  let last = -1;

  let userAvatarUrl = $derived($user?.avatarUrl ?? "/anonymous.webp");

  let start = $state();

  let comments = $state({
    current: "",
    initialized: false,
    fetching: false,
    sending: false,
    endReached: false,
    lastId: 0,
    data: [],
  });

  let textarea = $state();
  let tab = $state("write");

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
        ? `/api/posts/id/${postId}/comments?limit=${limit}`
        : `/api/posts/id/${postId}/comments?limit=${limit}&before=${comments.lastId}`;
    const res = await fetch(api);
    if (res.ok) {
      const data = await res.json();
      data.comments.forEach((comment) => {
        comment.content = md.render(comment.content);
      });
      updateComments(data.comments);
    }
    comments.fetching = false;
  };

  $effect(() => {
    if (last !== postId) {
      last = postId;

      comments = {
        current: "",
        initialized: false,
        fetching: false,
        sending: false,
        endReached: false,
        lastId: 0,
        data: [],
      };

      const onScrolled = gsap.to(start, {
        scrollTrigger: {
          trigger: start,
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
    }
  });
</script>

<section class="w-full xl:w-[calc(100%-15rem)] bg-white p-4 rounded-xl">
  <h4 class="text-lg lg:text-2xl">Join the discussion!</h4>
  <div class="flex flex-col gap-4" bind:this={start}>
    <hr class="border-t-3 border-dark mb-6" />
    <div class="flex gap-8">
      <div
        class="w-12 lg:w-20 h-12 lg:h-20 outline-primary outline-3 rounded-full overflow-hidden"
      >
        <img
          class="full object-cover"
          src={userAvatarUrl}
          alt="comment-posting-avatar"
        />
      </div>
      <div class="grow flex flex-col gap-4 relative">
        <svg
          class="absolute fill-primary top-6 lg:top-10 -left-4 -translate-y-1/2 w-4 h-4"
          viewBox="0 0 24 24"
        >
          <polygon points="0,12 24,0 24,24" /></svg
        >
        <div class="w-full">
          <div
            class="relative text-base w-full bg-primary-20 border-primary border-2 border-b-0 rounded-t-xl"
          >
            {#if tab === "preview"}
              <div class="w-full min-h-16 lg:min-h-20 overflow-hidden p-2">
                <Comment content={md.render(comments.current)} />
              </div>
            {:else if tab === "write"}
              <textarea
                name="comment-input"
                class="block w-full min-h-16 lg:min-h-20 overflow-hidden outline-none resize-none p-2"
                bind:this={textarea}
                bind:value={comments.current}
                {@attach autoHResize}
              >
              </textarea>
            {/if}
          </div>
          <div
            class="comment-editor flex xs:flex-col justify-between min-h-8 bg-primary rounded-b-xl"
          >
            <div class="flex">
              <button
                class="w-fit h-8 px-2 bg-primary-20 border-2 border-primary border-t-0 rounded-b-xl"
                class:z-11={tab === "write"}
                class:z-9={tab !== "write"}
                class:opacity-100={tab === "write"}
                class:opacity-90={tab !== "write"}
                onclick={() => (tab = "write")}
              >
                Write
              </button>
              <button
                class="w-fit h-8 px-2 -translate-x-0.5 bg-primary-20 border-2 border-primary border-t-0 rounded-b-xl"
                class:z-11={tab === "preview"}
                class:z-9={tab !== "preview"}
                class:opacity-100={tab === "preview"}
                class:opacity-90={tab !== "preview"}
                onclick={() => (tab = "preview")}
              >
                Preview
              </button>
            </div>
            {#if tab === "write"}
              <div
                class="flex ml-auto h-full my-auto *:bg-primary fill-white *:w-8 *:h-8 *:*:mx-auto *:hover:brightness-120 *:active:*:translate-y-0.5 gap-2 mr-2"
                in:fade={{ duration: 100 }}
              >
                <button
                  title="Header"
                  onclick={() => {
                    if (!textarea) return;
                    const start = textarea.selectionStart;
                    const end = textarea.selectionEnd;
                    textarea.value =
                      textarea.value.slice(0, start) +
                      "# " +
                      textarea.value.slice(start);

                    requestAnimationFrame(() => {
                      textarea.setSelectionRange(start + 2, end + 2);
                      textarea.focus();
                    });
                  }}
                >
                  <svg class="w-4 h-4" viewBox="0 0 16 16">
                    <path
                      d="M3.75 2a.75.75 0 0 1 .75.75V7h7V2.75a.75.75 0 0 1 1.5 0v10.5a.75.75 0 0 1-1.5 0V8.5h-7v4.75a.75.75 0 0 1-1.5 0V2.75A.75.75 0 0 1 3.75 2Z"
                    >
                    </path>
                  </svg>
                </button>
                <button
                  title="Bold"
                  onclick={() => {
                    if (!textarea) return;
                    const start = textarea.selectionStart;
                    const end = textarea.selectionEnd;
                    textarea.value =
                      textarea.value.slice(0, start) +
                      "**" +
                      textarea.value.slice(start, end) +
                      "**" +
                      textarea.value.slice(end);
                    requestAnimationFrame(() => {
                      textarea.setSelectionRange(start + 2, end + 2);
                      textarea.focus();
                    });
                  }}
                >
                  <svg class="w-4 h-4" viewBox="0 0 16 16">
                    <path
                      d="M4 2h4.5a3.501 3.501 0 0 1 2.852 5.53A3.499 3.499 0 0 1 9.5 14H4a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Zm1 7v3h4.5a1.5 1.5 0 0 0 0-3Zm3.5-2a1.5 1.5 0 0 0 0-3H5v3Z"
                    ></path>
                  </svg>
                </button>
                <button
                  title="Italic"
                  onclick={() => {
                    if (!textarea) return;
                    const start = textarea.selectionStart;
                    const end = textarea.selectionEnd;
                    textarea.value =
                      textarea.value.slice(0, start) +
                      "_" +
                      textarea.value.slice(start, end) +
                      "_" +
                      textarea.value.slice(end);
                    requestAnimationFrame(() => {
                      textarea.setSelectionRange(start + 1, end + 1);
                      textarea.focus();
                    });
                  }}
                >
                  <svg class="w-4 h-4" viewBox="0 0 16 16">
                    <path
                      d="M6 2.75A.75.75 0 0 1 6.75 2h6.5a.75.75 0 0 1 0 1.5h-2.505l-3.858 9H9.25a.75.75 0 0 1 0 1.5h-6.5a.75.75 0 0 1 0-1.5h2.505l3.858-9H6.75A.75.75 0 0 1 6 2.75Z"
                    ></path>
                  </svg>
                </button>
                <button
                  title="Code"
                  onclick={() => {
                    if (!textarea) return;
                    const start = textarea.selectionStart;
                    const end = textarea.selectionEnd;
                    textarea.value =
                      textarea.value.slice(0, start) +
                      "`" +
                      textarea.value.slice(start, end) +
                      "`" +
                      textarea.value.slice(end);
                    requestAnimationFrame(() => {
                      textarea.setSelectionRange(start + 1, end + 1);
                      textarea.focus();
                    });
                  }}
                >
                  <svg class="w-4 h-4" viewBox="0 0 16 16">
                    <path
                      d="m11.28 3.22 4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.749.749 0 0 1-1.275-.326.749.749 0 0 1 .215-.734L13.94 8l-3.72-3.72a.749.749 0 0 1 .326-1.275.749.749 0 0 1 .734.215Zm-6.56 0a.751.751 0 0 1 1.042.018.751.751 0 0 1 .018 1.042L2.06 8l3.72 3.72a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L.47 8.53a.75.75 0 0 1 0-1.06Z"
                    ></path>
                  </svg>
                </button>
              </div>
            {/if}
          </div>
        </div>
        <div class="ml-auto w-fit duo-btn duo-blue">
          <button
            class="fill-white"
            type="button"
            disabled={comments.sending || comments.current.length < 1}
            onclick={async () => {
              const headers =
                $user !== undefined
                  ? {
                      Authorization: auth(),
                      "Content-Type": "application/json",
                    }
                  : {
                      "Content-Type": "application/json",
                    };

              comments.sending = true;

              const res = await fetch(`/api/posts/id/${postId}/comments/new`, {
                method: "PUT",
                headers,
                body: JSON.stringify({
                  content: comments.current,
                }),
              });
              if (res.ok) {
                const data = await res.json();
                const userData =
                  $user !== undefined
                    ? {
                        display_name: $user.displayName,
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
                newComment.content = md.render(newComment.content);
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
    {#each comments.data as { id, avatar_url: url, content, created_at: createdAt, display_name: displayName, username, user_role: userRole }, index (id)}
      {@const anonymous = username !== undefined && displayName}
      <li in:fly={{ y: 10, duration: 500 }}>
        <div class="flex py-4">
          <div
            class="ml-2 min-w-10 lg:min-w-12 w-10 lg:w-12 h-10 lg:h-12 outline-primary outline-3 rounded-full shadow-md overflow-hidden"
          >
            {#if anonymous}
              <a class="full" href={"/profiles/" + username}>
                <img
                  class="full object-cover"
                  src={url ?? "/anonymous.webp"}
                  alt="comment-posting-avatar"
                />
              </a>
            {:else}
              <img
                class="full object-cover"
                src={url ?? "/anonymous.webp"}
                alt="comment-posting-avatar"
              />
            {/if}
          </div>
          <div class="relative flex flex-col grow">
            <div class="pl-2 -translate-y-2">
              <div class="relative w-fit">
                <div class="w-fit p-2 bg-primary/20 rounded-2xl rounded-tl-md">
                  <div class="flex items-center lg:text-base">
                    {#if anonymous}
                      <a class="font-semibold" href={"/profiles/" + username}
                        >{displayName}
                      </a>
                    {:else}
                      <span class="font-normal select-none italic"
                        >Anonymous</span
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

                  <Comment {content} />
                </div>
                <div
                  class="absolute flex min-w-20 w-full justify-between gap-2 left-0 top-full text-sm"
                >
                  <span class="pl-2">{createdAt ?? "new"}</span>
                  <div class="text-nowrap *:cursor-pointer">
                    <span>reply</span>
                    <span>report</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </li>
    {/each}
  </ul>
  <div class="mt-8 mx-auto w-fit duo-btn duo-blue">
    <button
      disabled={comments.endReached || comments.fetching}
      onclick={fetchComments}
      >{comments.endReached ? "No more to read ૮´•˕•`ა" : "Read more"}</button
    >
  </div>
</section>

<style lang="postcss">
  @reference "../../../app.css";

  .comment-editor {
    @apply relative *:relative;
    &::before {
      @apply z-10 absolute! content-[''] w-full h-full top-0 left-0 border-2 border-primary rounded-b-xl pointer-events-none;
    }
  }
</style>
