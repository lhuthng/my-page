<script>
  import { auth } from "$lib/client/user";
  import { useDebounce } from "$lib/effects/debounce";
  import SearchButton from "../buttons/SearchButton.svelte";
  import MediumEntity from "./MediumEntity.svelte";

  let { ...rest } = $props();

  let cache = $state({});

  let keyword = $state("");
  let _keyword = $state("");

  const keywordDebounce = useDebounce(async (_keyword) => {
    keyword = _keyword;
  }, 300);

  const search = async (keyword) => {
    if (keyword.length < 2) return;

    if (cache[keyword] === undefined) {
      const request = { status: "fetching" };
      cache[keyword] = { ...request };

      const res = await fetch(`/api/media?term=${keyword}&size=10`, {
        method: "GET",
        headers: { Authorization: auth() },
      });

      if (res.ok) {
        request.status = "success";
        request.results = (await res.json()).results;
      } else {
        request.status = "failed";
      }

      cache[keyword] = { ...request };
    }
  };

  $effect(() => {
    keywordDebounce.update(_keyword);
  });

  $effect(() => {
    if (cache[keyword] === undefined) search(keyword);
  });
</script>

{#snippet printResults(cache, keyword)}
  {@const results = cache[keyword]?.results}
  <div class="p-2 grow min-h-0 max-h-full custom-scrollbar">
    {#if results !== undefined}
      {#if results.length === 0}
        <span class="block text-center"
          >There is no medium matching "{keyword}"</span
        >
      {:else}
        <ul class="space-y-2">
          {#each results as { short_name: shortName, url, file_type: type }}
            <MediumEntity {shortName} {url} {type} />
          {/each}
        </ul>
      {/if}
    {:else}
      <span class="block text-center"
        >Enter atleast 2 characters to search media.</span
      >
    {/if}
  </div>
{/snippet}

<div {...rest}>
  <div class="mx-auto w-full p-2">
    <div
      class="flex items-center rounded-full w-full h-8 px-2 border-2 border-primary"
    >
      <input
        id="media-searcher"
        class="outline-none grow"
        autocorrect="off"
        bind:value={_keyword}
      />
      <SearchButton
        class="w-6 h-6 not-disabled:hover:scale-110 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50 fill-primary"
      />
    </div>
  </div>
  {@render printResults(cache, keyword)}
</div>
