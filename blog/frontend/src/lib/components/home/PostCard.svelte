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
    children,
  } = $props();

  let toggled = $state(false);

  let link = $derived(
    dashboardMode ? `/dashboard/posts/${id}` : `/posts/${slug}`,
  );

  let postLink = $derived(`/posts/${slug}`);
</script>

<div
  class="relative flex gap-4 bg-background/40 hover:bg-background/60 transition-colors duration-50 rounded-lg"
>
  {#if !dashboardMode}
    <a
      class="relative block z-10 min-w-26 min-h-26 md:min-w-34 md:min-h-34"
      href={status === "draft" ? `/dashboard/posts/id/${id}` : link}
    >
      <img
        class="absolute z-10 left-0 top-0 w-26 h-26 md:w-34 md:h-34 object-cover rounded-lg origin-center transition-transform duration-100 cursor-pointer hover:scale-105"
        src={src ?? "/missing.png"}
        alt="post-cover"
      />
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
  <div class="relative z-10 w-full pointer-events-none overflow-hidden">
    <div
      class="card absolute w-[200%] h-full flex transition-transform duration-200 left-0"
      class:toggled
    >
      <div class="h-full w-1/2 min-w-0">
        <div class="flex flex-col full py-2 min-w-0">
          <a
            class="w-fit"
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
          <div class="flex gap-1 grow shrink text-sm sm:text-md line-clamp-2">
            {#if tags?.length > 0}
              <span>tags: </span>
            {/if}
            <ul class="tag-container flex flex-wrap h-fit gap-y-1 gap-x-1 pr-2">
              {#each tags as tag}
                <li>
                  <a href={`/tags/${tag.replace(" ", "-")}`}>#{tag}</a>
                </li>
              {/each}
            </ul>
          </div>
          <div class="flex justify-between items-center text-base pr-4">
            <div class="flex gap-1 items-center text-md">
              <!-- <CommentButton
                                as="span"
                                class="block w-6 h-6 fill-dark"
                            />
                            <span class="pointer-events-auto">6</span> -->
            </div>
          </div>
        </div>
      </div>
      <div class="absolute left-1/2 h-full w-1/2" class:toggled>
        <div
          class="full p-2 pointer-events-auto text-xs sm:text-base overscroll-contain custom-scrollbar overflow-y-scroll"
        >
          <p>{excerpt}</p>
          <a
            class="block text-right"
            href={status === "draft"
              ? `/dashboard/posts/id/${id}`
              : !dashboardMode
                ? link
                : undefined}>> to post</a
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
