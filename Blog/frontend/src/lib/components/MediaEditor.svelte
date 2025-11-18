<script>
    import { getAuthorization } from "$lib/client/user";
    import { useDebounce } from "$lib/effects/debounce";
    import MediaEntity from "./MediaEntity.svelte";
    import Portal from "./Portal.svelte";

    let { detailPanel, searchKeywords } = $props();

    let debouncedKeywords = $state(searchKeywords);
    let selectedName = $state();
    const maxSize = 10;

    let cachedRequests = $state({});
    let cachedDetails = $state({});
    let details = $derived(cachedDetails[selectedName]);

    let status = $derived(cachedRequests[debouncedKeywords]?.status ?? "hold");
    let items = $derived(cachedRequests[debouncedKeywords]?.results ?? []);

    let debounce = useDebounce(async (searchKeywords) => {
        debouncedKeywords = searchKeywords;
        if (searchKeywords === "") return;
    	if (!cachedRequests[searchKeywords]) {
            const request = {
                status: "waiting"
            };
            cachedRequests[searchKeywords] = { ...request };
            try {
                const res = await fetch(`/api/media?term=${searchKeywords}&size=${maxSize}&skip=0`, {
                    method: "GET",
                    headers: { Authorization: getAuthorization() }
                });
                if (res.ok) {
                    request.status = "success";
                    request.results = (await res.json()).results;
                    cachedRequests[searchKeywords] = { ...request };
                }
                else {
                    request.status = "failed";
                }
            } catch (e) {
                console.error(e);
            }
     	}
    }, 1000);

    let loadDetails = async (short_name) => {
        if (!cachedDetails[short_name]) {
            const details = { status: "waiting" };
            cachedDetails[short_name] = { ...details };
            try {
                const res = await fetch(`/api/media/d/${short_name}`, {
                    method: "GET",
                    headers: { Authorization: getAuthorization() }
                });
                if (res.ok) {
                    details.status = "success"
                    details.result = await res.json();
                }
                else {
                    details.status = "failed"
                }
                cachedDetails[short_name] = { ... details };
            } catch (e) {
                console.error(e);
            }
        }
    }

    $effect(() => {
        let keywords = searchKeywords;
        debounce.update(searchKeywords);
        return () => debounce.destroy();
    })
</script>
<div class="flex">
    {#if status === "success"}
        {#each items as item, index (item.short_name)}
            <MediaEntity
                size={120}
                file={{name: item.short_name, url: item.url}}
                isSelected={selectedName===item.short_name}
                onclick={() => loadDetails(selectedName = item.short_name)}
            />
        {/each}
    {/if}
</div>
<Portal class="p-2" target={detailPanel}>
    {#if !details}
        <span>Search and select any media to edit the file</span>
    {:else if details.status === "waiting"}
        <span>Waiting, I'm loading</span>
    {:else}
        <div class="flex flex-col gap-2">
            <span>Short name: {details.result.short_name}</span>
            <span>Description: {details.result.description}</span>
            <span>File type: {details.result.file_type}</span>
        </div>
    {/if}
</Portal>
