<script>
    let { shortName, url, warning, handlers } = $props();
    let draft = $state(shortName);
    let skip = false;
</script>

<li
    class={`flex items-center gap-2 p-2 rounded-lg ${warning ? "bg-yellow-200" : "bg-gray-300"} hover:brightness-105`}
>
    <div class="flex w-10 h-10">
        <img
            class="m-auto max-w-10 max-h-10 object-contain rounded-sm overflow-hidden"
            src={url}
            alt={shortName}
        />
    </div>
    {#if handlers?.changeName}
        <input
            class="focus:outline-none focus:border-b"
            type="text"
            bind:value={draft}
            onblur={() => {
                if (!skip) {
                    draft = shortName;
                }
                skip = false;
            }}
            onkeydown={(e) => {
                if (e.key === "Enter") {
                    if (handlers.changeName(shortName, draft)) {
                        skip = true;
                    }
                    e.target.blur();
                }
            }}
        />
    {:else}
        <span>{shortName}</span>
    {/if}
</li>
