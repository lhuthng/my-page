<script>
  import { dateTillNow, textToDate } from "$lib/common";
  import PostSection from "$lib/components/post/PostSection.svelte";
  import CommentSection from "$lib/components/post/CommentSection.svelte";
  import { page } from "$app/state";
  import { isLiked, shouldSendView, VIEW_DELAY } from "$lib/client/post.js";
  import { browser } from "$app/environment";

  let { data, slug } = $props();

  let {
    id,
    author_slug,
    author_name,
    author_avatar_url,
    title,
    content,
    excerpt,
    published_at,
    updated_at,
    tags,
    series,
    cover_url,
  } = $derived(data);
  let liked = $state();
  $effect(() => {
    liked = browser ? isLiked(id) : false;
  });
  let date = $derived(textToDate(published_at));
  let updateTime = $derived(dateTillNow(updated_at, "round"));

  let imageUrl = $derived(page.url.origin + cover_url);
  let canonicalLink = $derived.by(() => {
    const { origin, pathname, search } = page.url;
    return origin + pathname + search;
  });

  let viewDelayTimeout = null;
  $effect(() => {
    if (viewDelayTimeout) {
      clearTimeout(viewDelayTimeout);
    }

    viewDelayTimeout = setTimeout(async () => {
      if (shouldSendView(id)) {
        const res = await fetch(`/api/posts/id/${id}/view`, { method: "POST" });
      }
    }, VIEW_DELAY);

    return () => {
      if (viewDelayTimeout) {
        clearTimeout(viewDelayTimeout);
        viewDelayTimeout = null;
      }
    };
  });
</script>

<svelte:head>
  <title>{title}</title>
  <meta name="description" content={excerpt} />

  <meta property="og:title" content={title} />
  <meta property="og:type" content="article" />
  <meta property="og:url" content={canonicalLink} />
  <meta property="og:description" content={excerpt} />
  <meta property="og:image" content={imageUrl} />

  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:url" content={canonicalLink} />
  <meta name="twitter:title" content={title} />
  <meta name="twitter:description" content={excerpt} />
  <meta name="twitter:image" content={imageUrl} />

  <link rel="canonical" href={canonicalLink} />
</svelte:head>

<article class="flex flex-col gap-4 pb-4 *:drop-shadow-xl">
  <PostSection
    {id}
    {title}
    {tags}
    {date}
    {updateTime}
    {content}
    {series}
    {liked}
    author={{
      username: author_slug,
      displayName: author_name,
      avatarUrl: author_avatar_url,
    }}
  />

  <CommentSection postId={id} />
</article>
