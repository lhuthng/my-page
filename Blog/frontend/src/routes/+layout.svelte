<script>
    import { page } from "$app/stores";
    import { saveLogin } from "$lib/client/user";
    import Footer from "$lib/components/Footer.svelte";
    import Header from "$lib/components/Header.svelte";
    import NavigationSideBar from "$lib/components/NavigationSideBar.svelte";
    import "../app.css";

    let { children } = $props();

    let route = $derived($page.url.pathname.split("/")[1]);
</script>

<div class="relative flex flex-col min-h-screen">
    <Header />
    <main class="grow">
        <div class="relative flex gap-2 lg:gap-4 w-cap">
            {#if route !== "login"}
                <NavigationSideBar {route} />
            {/if}
            <div class="w-full">
                {@render children?.()}
            </div>
        </div>
    </main>
    <Footer />
</div>

<style lang="postcss">
    @reference "../app.css";

    main {
        @apply flex pt-32 not-lg:pt-26.5 bg-background text-dark;
    }
</style>
