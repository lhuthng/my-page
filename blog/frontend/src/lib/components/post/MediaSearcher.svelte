<script>
  import { auth } from "$lib/client/user";
  import { useDebounce } from "$lib/effects/debounce";
  import SearchButton from "../buttons/SearchButton.svelte";
  import MediumEntity from "./MediumEntity.svelte";

  let { ...rest } = $props();

  let requestCache = $state({});

  let keyword = $state("");
  let deKeyword = $state("");

  let results = $derived(requestCache[deKeyword]?.results);

  let debounce = useDebounce(async (keyword) => {
    deKeyword = keyword;
  }, 300);

  async function search(keyword) {
    if (keyword.length < 2) return;
    if (requestCache[keyword] === undefined) {
      const req = { status: "waiting" };
      requestCache[keyword] = { ...req };

      const res = await fetch(`/api/media?term=${keyword}&size=10`, {
        method: "GET",
        headers: { Authorization: auth() },
      });

      if (res.ok) {
        req.status = "success";
        req.results = (await res.json()).results;
      } else {
        req.status = "failed";
      }
      requestCache[keyword] = { ...req };
    }
  }

  $effect(() => {
    debounce.update(keyword);
    return () => debounce.destroy();
  });

  $effect(() => {
    search(deKeyword);
  });
</script>

<div {...rest}>
  <div class="mx-auto p-4">
    <div
      class="flex items-center rounded-full w-full h-8 px-2 border border-gray-400"
    >
      <input
        class="outline-none grow"
        bind:value={keyword}
        autocomplete="off"
        autocorrect="off"
        writingsuggestions="false"
        min="3"
      />
      <SearchButton
        class="w-6 h-6 not-disabled:hover:scale-110 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
        fill="gray"
      />
    </div>
  </div>
  <div class="grow p-2 overflow-y-scroll">
    {#if results !== undefined}
      {#if results.length === 0}
        <span class="block text-center"
          >There is no medium matching "{deKeyword}"</span
        >
      {:else}
        <ul class="space-y-2">
          {#each results as { short_name: shortName, url }}
            <MediumEntity {shortName} {url} />
          {/each}
        </ul>
      {/if}
    {:else}
      <span class="block text-center"
        >Enter atleast 2 characters to search media.</span
      >
    {/if}
  </div>
</div>
