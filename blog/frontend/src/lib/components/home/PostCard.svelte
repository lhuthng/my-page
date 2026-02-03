<script>
  import CommentButton from "../buttons/CommentButton.svelte";

  let {
    src,
    id,
    status,
    dashboardMode,
    onclick,
    title,
    slug,
    series,
    excerpt,
    author,
    tags,
    previewMode,
    stats,
    children,
  } = $props();

  let toggled = $state(false);

  let link = $derived(
    dashboardMode ? `/dashboard/posts/${id}` : `/posts/${slug}`,
  );

  let postLink = $derived(`/posts/${slug}`);
</script>

<div class="bg-white rounded-lg drop-shadow-sm">
  <div
    class="relative flex gap-4 bg-background/40 hover:bg-background/60 transition-colors duration-50 rounded-lg"
  >
    {#if !dashboardMode}
      <a
        class="relative block z-10 min-w-26 min-h-26 md:min-w-34 md:min-h-34 cursor-pointer rounded-lg origin-center hover:scale-105 transition-transform duration-100 overflow-hidden"
        href={status === "draft" ? `/dashboard/posts/id/${id}` : link}
      >
        <img
          class="absolute z-10 left-0 top-0 w-26 h-26 md:w-34 md:h-34 object-cover"
          src={src ?? "/missing.png"}
          alt="post-cover"
        />
        <div
          class="absolute flex items-center justify-center z-11 left-0 right-0 bottom-0 h-8 bg-dark/80 text-sm font-semibold"
        >
          <div>
            <span class="text-white/80!">{stats?.views ?? "#"}</span>
            <svg class="inline-block fill-white/80 h-6" viewBox="0 0 24 24">
              <path
                d="M11.5 6C10.9477 6 10.5 6.44772 10.5 7C10.5 7.55228 10.9477 8 11.5 8H20C20.5523 8 21 7.55228 21 7C21 6.44772 20.5523 6 20 6H11.5ZM15 11C14.4477 11 14 11.4477 14 12C14 12.5523 14.4477 13 15 13H20C20.5523 13 21 12.5523 21 12C21 11.4477 20.5523 11 20 11H15ZM12 16C11.4477 16 11 16.4477 11 17C11 17.5523 11.4477 18 12 18H20C20.5523 18 21 17.5523 21 17C21 16.4477 20.5523 16 20 16H12ZM7.70711 8.29289C7.31658 7.90237 6.68342 7.90237 6.29289 8.29289C5.90237 8.68342 5.90237 9.31658 6.29289 9.70711L7.58579 11H4C3.44772 11 3 11.4477 3 12C3 12.5523 3.44772 13 4 13H7.58579L6.29289 14.2929C5.90237 14.6834 5.90237 15.3166 6.29289 15.7071C6.68342 16.0976 7.31658 16.0976 7.70711 15.7071L10.7071 12.7071C11.0976 12.3166 11.0976 11.6834 10.7071 11.2929L7.70711 8.29289Z"
              ></path>
            </svg>
          </div>
          <div>
            <svg class="inline-block fill-white h-6" viewBox="0 0 24 24"
              ><path
                fill-rule="evenodd"
                clip-rule="evenodd"
                d="M4 12C4 7.58172 7.58172 4 12 4C16.4183 4 20 7.58172 20 12C20 16.4183 16.4183 20 12 20C7.58172 20 4 16.4183 4 12ZM12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22C17.5228 22 22 17.5228 22 12C22 6.47715 17.5228 2 12 2ZM9.96665 14.7439C9.827 14.2162 9.28872 13.897 8.75746 14.0299C8.22166 14.1638 7.8959 14.7067 8.02985 15.2425C8.46886 16.9986 10.2833 18 12 18C13.7853 18 15.4524 16.9881 15.9701 15.24C16.1041 14.7042 15.7783 14.1638 15.2425 14.0299C14.7113 13.897 14.173 14.2162 14.0333 14.7439C13.7722 15.6146 12.8398 16 12 16C11.1602 16 10.2278 15.6147 9.96665 14.7439ZM13 11C13 10.4477 13.4477 10 14 10H16C16.5523 10 17 10.4477 17 11C17 11.5523 16.5523 12 16 12H14C13.4477 12 13 11.5523 13 11ZM9.5 12C10.3284 12 11 11.3284 11 10.5C11 9.67157 10.3284 9 9.5 9C8.67157 9 8 9.67157 8 10.5C8 11.3284 8.67157 12 9.5 12Z"
              ></path></svg
            >
            <span class="text-white/80!">{stats?.likes ?? "#"}</span>
          </div>
        </div>
      </a>
    {:else}
      <button
        class="relative block z-10 min-w-26 min-h-26 md:min-w-34 md:min-h-34"
        {onclick}
      >
        <img
          class="absolute z-10 left-0 top-0 w-26 h-26 md:w-34 md:h-34 object-cover rounded-lg origin-center transition-transform duration-100 cursor-pointer hover:scale-105"
          src={src ?? "/missing.png"}
          alt="post-cover"
        />
        {#if children !== undefined}
          {@render children()}
        {/if}
      </button>
    {/if}
    <div
      class="relative z-10 w-full pointer-events-none rounded-r-lg overflow-hidden"
    >
      <div
        class="card absolute w-[200%] h-full flex transition-transform duration-200 left-0"
        class:toggled
      >
        <div class="h-full w-1/2 min-w-0">
          <div class="flex flex-col full py-2 min-w-0">
            <a
              class="w-fit pr-2"
              href={status === "draft"
                ? `/dashboard/posts/id/${id}`
                : !dashboardMode
                  ? link
                  : undefined}
              ><h1 class="text-md md:text-lg line-clamp-2">
                {title}
                {#if status === "draft"}
                  <i class="text-accent-red">(draft)</i>
                {:else if dashboardMode}
                  <i class="text-accent-red">(dashboard)</i>
                {/if}
              </h1>
            </a>
            <div class="flex text-sm sm:text-md pr-4">
              <span class="select-none pointer-events-auto"
                >by <a
                  class="select-text text-dark!"
                  href={!dashboardMode ? `/profiles/${author.slug}` : undefined}
                  >{author.name}</a
                ></span
              >
              {#if series !== undefined}
                <div class="flex grow shrink gap-2">
                  <span>;</span>
                  <span class="pointer-events-auto">
                    <span>::{series.order}</span>
                    from
                    <a class="text-dark!" href={`/series/${series.slug}`}
                      >{series.name}</a
                    >
                  </span>
                </div>
              {/if}
            </div>
            <div class="flex text-sm sm:text-md gap-1 grow shrink mb-2">
              {#if tags?.length > 0}
                <span>tags: </span>
              {/if}
              <ul
                class="tag-container flex flex-wrap h-fit gap-y-2 sm:gap-y-0.5 gap-x-1 pr-2"
              >
                {#each tags as tag}
                  <li>
                    <a href={`/tags/${tag.replace(" ", "-")}`}>#{tag}</a>
                  </li>
                {/each}
              </ul>
            </div>
          </div>
        </div>
        <div class="absolute left-1/2 h-full w-1/2" class:toggled>
          <div
            class="full p-2 pointer-events-auto text-sm sm:text-base overscroll-contain custom-scrollbar overflow-y-scroll"
          >
            <p>{excerpt}</p>
            <a
              class="block text-right"
              href={status === "draft"
                ? `/dashboard/posts/id/${id}`
                : !dashboardMode
                  ? link
                  : undefined}><span class="select-none">>{" "}</span>to post</a
            >
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
</div>

<style lang="postcss">
  @reference "../../../app.css";

  span {
    @apply text-dark/50;
  }

  .tag-container {
    @apply pointer-events-none;
    & > li {
      @apply h-4;
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
