<script>
  import { page } from "$app/stores";
  import { saveLogin } from "$lib/client/user";
  import Footer from "$lib/components/Footer.svelte";
  import Header from "$lib/components/Header.svelte";
  import NavigationSideBar from "$lib/components/NavigationSideBar.svelte";
  import { onMount } from "svelte";
  import "../app.css";
  import { mbody, pbody } from "$lib/client/misc";
  import { fade } from "svelte/transition";
  import ToTop from "$lib/components/ToTop.svelte";
  import { width } from "$lib/client/windows";
  import { innerWidth } from "svelte/reactivity/window";

  let { children } = $props();

  let route = $derived($page.url.pathname.split("/")[1]);
  let pDiv = $state();
  let mDiv = $state();
  let scrollTarget = $state();

  $effect(() => {
    width.set(innerWidth.current);
  });

  onMount(() => {
    pbody.set(pDiv);
    mbody.set(mDiv);
  });
</script>

<svelte:head>
  <meta property="og:site_name" content="Huu Thang's Blog" />

  <link rel="icon" href="/favicon.ico" />
</svelte:head>

<div class="fixed w-dvw h-dvh pointer-events-none z-11" bind:this={mDiv}></div>
<div class="relative flex flex-col min-h-screen z-10">
  <div class="absolute pointer-events-none inset-0 z-50" bind:this={pDiv}></div>
  <Header />
  <main class="grow" bind:this={scrollTarget}>
    <div class="relative flex gap-2 lg:gap-4 w-cap">
      {#if route !== "login"}
        <NavigationSideBar {route} />
      {/if}
      <div class="w-full not-lg:overflow-x-hidden">
        {@render children?.()}
      </div>
    </div>
    <ToTop {scrollTarget} />
  </main>
  <Footer />
</div>

<style lang="postcss">
  @reference "../app.css";

  main {
    @apply flex pt-32 not-lg:pt-16 bg-background text-dark;
  }
</style>
