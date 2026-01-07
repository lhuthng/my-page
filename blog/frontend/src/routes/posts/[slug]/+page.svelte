<script>
  import { textToDate } from "$lib/common";
  import PostSection from "$lib/components/post/PostSection.svelte";
  import CommentSection from "$lib/components/post/CommentSection.svelte";

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
  } = $derived(data);
  let date = $derived(textToDate(published_at));
</script>

<svelte:head>
  <title>{title}</title>
  <!-- <meta name="description" content={excerpt} /> -->
  <meta property="og:title" content={title} />
  <!-- <meta property="og:description" content={excerpt} /> -->
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
