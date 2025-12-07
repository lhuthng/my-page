<script>
    import HomeDiscovery from "$lib/components/home/HomeDiscovery.svelte";
    import Suggestion from "$lib/components/home/Suggestion.svelte";

    const { data } = $props();

    const featuredPosts = data.featured_posts || [];
</script>

<svelte:head>
    <title>Home</title>
    <!-- <meta name="description" content="Svelte demo app" /> -->
</svelte:head>

<div class="relative z-5 flex gap-4 *:h-fit">
    <div class="grow bg-white/90 rounded-xl px-4 py-2 overflow-hidden">
        <HomeDiscovery {featuredPosts} />
    </div>
    <div class="not-lg:hidden w-60 bg-white/90 rounded-xl">
        <Suggestion />
    </div>
</div>

<style lang="postcss">
    @reference "../app.css";
</style>
