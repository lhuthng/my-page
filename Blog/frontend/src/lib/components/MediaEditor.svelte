<script>
    import { detailsCache } from "$lib/client/details-cache";
    import { auth } from "$lib/client/user";
    import { useDebounce } from "$lib/effects/debounce";

    import MediaDirectory from "./MediaDirectory.svelte";
    import MediaEditForm from "./MediaEditForm.svelte";
    import MediaEntity from "./MediaEntity.svelte";
    import Portal from "./Portal.svelte";

    let { searchKeywords, detailPanel } = $props();

    let requestCache = $state({});
    let selection = $state();

    // Debounce the search keywords
    let deSearchKeywords = $state(searchKeywords);
    let debounce = useDebounce(async (searchKeywords) => {
        deSearchKeywords = searchKeywords;
        selection = undefined;
    }, 300);
    $effect(() => {
        debounce.update(searchKeywords);
        return () => debounce.destroy();
    });

    // Check for results
    async function search(keywords) {
        if (keywords === "") return;
        if (requestCache[keywords] === undefined) {
            const req = { status: "waiting" };
            requestCache[keywords] = { ...req };

            const res = await fetch(`/api/media?term=${keywords}&size=10`, {
                method: "GET",
                headers: { Authorization: auth() },
            });

            if (res.ok) {
                req.status = "success";
                req.results = (await res.json()).results;
            } else {
                req.status = "failed";
            }
            requestCache[keywords] = { ...req };
        }
    }
    $effect(async () => {
        search(deSearchKeywords);
    });
</script>

<div class="relative full">
    <MediaDirectory
        class="full p-2"
        cellWidth="120px"
        cellHeight="200px"
        onclick={() => {}}
    >
        {#if requestCache[deSearchKeywords]?.status === "success"}
            {#each requestCache[deSearchKeywords]?.results as item, index (item.short_name)}
                <MediaEntity
                    size={80}
                    file={{ name: item.short_name, url: item.url }}
                    isSelected={selection === item.short_name}
                    onclick={async () => {
                        if (!$detailsCache[item.short_name]) {
                            const details = { status: "waiting" };
                            $detailsCache[item.short_name] = { ...details };
                            const res = await fetch(
                                `/api/media/d/${item.short_name}`,
                                {
                                    method: "GET",
                                    headers: {
                                        Authorization: auth(),
                                    },
                                },
                            );
                            if (res.ok) {
                                details.status = "success";
                                details.result = await res.json();
                            } else {
                                details.status = "failed";
                            }
                            $detailsCache[item.short_name] = { ...details };
                        }
                        selection = item.short_name;
                    }}
                    ondblclick={() => {}}
                />
            {/each}
        {/if}
    </MediaDirectory>
</div>
<Portal class="p-2" target={detailPanel}>
    <MediaEditForm
        shortName={selection}
        onShortNameChanged={(newShortName) => {
            delete requestCache[deSearchKeywords];
            search(deSearchKeywords);
            selection = undefined;
        }}
    />
</Portal>
