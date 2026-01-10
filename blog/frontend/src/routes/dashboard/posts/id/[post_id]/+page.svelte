<script>
  import PostEditor from "$lib/components/post/PostEditor.svelte";

  const { data } = $props();

  const {
    id,
    title,
    slug,
    series,
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
