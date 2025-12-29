<script>
    let { shortName, url, warning, changeName } = $props();
    let draft = $state(shortName);
    let skip = false;
</script>

<li
    class="flex items-center gap-2 p-2 rounded-lg bg-primary/20 hover:brightness-105"
    class:bg-yellow-200={warning}
>
    <div class="flex w-8 lg:w-10 h-8 lg:h-10">
        <img
            class="m-auto max-w-8 lg:max-w-10 max-h-8 lg:max-h-10 object-contain rounded-sm overflow-hidden"
            src={url}
            alt={shortName}
        />
    </div>
    {#if changeName}
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
                    if (changeName(shortName, draft)) {
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
