<script>
    import { detailsCache } from "$lib/client/details-cache";
    import { auth } from "$lib/client/user";
    import { useDebounce } from "$lib/effects/debounce";

    import MediaDirectory from "./MediaDirectory.svelte";
    import MediaEditForm from "./MediaEditForm.svelte";
    import MediumEntity from "./MediumEntity.svelte";
    import Portal from "$lib/components/Portal.svelte";

    let { keyword, detailPanel } = $props();

    let requestCache = $state({});
    let selection = $state();

    // Debounce the search keywords
    let deKeyword = $state(keyword);
    let debounce = useDebounce(async (searchKeywords) => {
        deKeyword = searchKeywords;
        selection = undefined;
    }, 300);

    // Check for results
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

    $effect(async () => {
        search(deKeyword);
    });
</script>

<div class="relative full">
    <MediaDirectory
        class="full p-2"
        cellWidth="120px"
        cellHeight="200px"
        onclick={() => {}}
    >
        {#if requestCache[deKeyword]?.status === "success"}
            {#each requestCache[deKeyword]?.results as item, index (item.short_name)}
                <MediumEntity
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
            delete requestCache[deKeyword];
            search(deKeyword);
            selection = undefined;
        }}
    />
</Portal>
