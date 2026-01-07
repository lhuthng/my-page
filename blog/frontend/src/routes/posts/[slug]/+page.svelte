<script>
  import { textToDate } from "$lib/common";
  import PostSection from "$lib/components/post/PostSection.svelte";
  import CommentSection from "$lib/components/post/CommentSection.svelte";
  import { page } from "$app/state";

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
    tags,
    cover_url,
  } = $derived(data);
  let date = $derived(textToDate(published_at));

  let imageUrl = $derived(page.url.origin + cover_url);
</script>

<svelte:head>
  <title>{title}</title>
  <meta name="description" content={excerpt} />

  <meta property="og:title" content={title} />
  <meta property="og:type" content="article" />
  <meta property="og:url" content={page.url.href} />
  <meta property="og:description" content={excerpt} />
  <meta property="og:image" content={imageUrl} />

  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:url" content={page.url.href} />
  <meta name="twitter:title" content={title} />
  <meta name="twitter:description" content={excerpt} />
  <meta name="twitter:image" content={imageUrl} />

  <link rel="canonical" href={page.url.href} />
</svelte:head>

<article class="flex flex-col gap-4 pb-4 *:drop-shadow-xl">
  <PostSection
    {id}
    {title}
    {tags}
    {date}
    {content}
    author={{
      username: author_slug,
      displayName: author_name,
      avatarUrl: author_avatar_url,
    }}
  />

  <CommentSection postId={id} />
</article>
