<script>
    import HomeDiscovery from "$lib/components/home/HomeDiscovery.svelte";
    import Introduction from "$lib/components/home/Introduction.svelte";
    import Suggestion from "$lib/components/home/Suggestion.svelte";

    const { data } = $props();

    const featuredPosts = data.featured_posts || [];
</script>

<svelte:head>
    <title>Home</title>
    <!-- <meta name="description" content="Svelte demo app" /> -->
</svelte:head>

<div class="relative z-5 flex gap-4 *:h-fit pb-2 lg:pb-4">
    <div class="grow space-y-2 lg:space-y-4">
        <div class="bg-white/90 rounded-xl px-4 overflow-hidden">
            <HomeDiscovery {featuredPosts} />
        </div>
    </div>
    <div class="not-lg:hidden w-60 bg-white/90 rounded-xl">
        <Suggestion />
    </div>
</div>
<div class="pb-4">
    <Introduction />
</div>

<style lang="postcss">
    @reference "../app.css";
</style>
