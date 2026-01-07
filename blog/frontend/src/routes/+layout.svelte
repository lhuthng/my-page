<script>
  import { page } from "$app/stores";
  import { saveLogin } from "$lib/client/user";
  import Footer from "$lib/components/Footer.svelte";
  import Header from "$lib/components/Header.svelte";
  import NavigationSideBar from "$lib/components/NavigationSideBar.svelte";
  import { onMount } from "svelte";
  import "../app.css";
  import { pbody } from "$lib/client/misc";
  import { fade } from "svelte/transition";

  let { children } = $props();

  let route = $derived($page.url.pathname.split("/")[1]);
  let specialDiv = $state();

  onMount(() => {
    pbody.set(specialDiv);
  });
</script>

<div class="relative flex flex-col min-h-screen">
  <div
    class="absolute pointer-events-none inset-0 z-50"
    transition:fade={{ duration: 0 }}
    bind:this={specialDiv}
  ></div>
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
    @apply flex pt-32 not-lg:pt-16 bg-background text-dark;
  }
</style>
