<script>
    import PostEditor from "$lib/components/post/PostEditor.svelte";

    const { data } = $props();

    const {
        id,
        title,
        slug,
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
        content,
        draft,
        tags,
        excerpt,
        mediumShortNames,
        mediumUrls,
    }}
/>
