<script>
  import PostEditor from "$lib/components/post/PostEditor.svelte";

  const { data } = $props();

  const {
    id,
    title,
    slug,
    series,
    cover_url: coverUrl,
    series_slug: seriesSlug,
    series_cover_url: seriesCoverUrl,
    content,
    draft,
    tags,
    excerpt,
    medium_short_names: mediumShortNames,
    medium_urls: mediumUrls,
  } = data;
</script>

<PostEditor
  mode="edit"
  data={{
    id,
    title,
    slug,
    coverUrl,
    series,
    seriesSlug,
    seriesCoverUrl,
    content,
    draft,
    tags,
    excerpt,
    mediumShortNames,
    mediumUrls,
  }}
/>
