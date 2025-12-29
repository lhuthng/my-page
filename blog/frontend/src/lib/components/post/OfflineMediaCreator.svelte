<script>
    import { preventDefault } from "$lib/common";
    import MediumEntity from "./MediumEntity.svelte";

    let { offlineMedia, onlineMedia, updateNewMedia, changeName, ...rest } =
        $props();

    function addMedia(files) {
        const media = [];
        for (let index = 0; index < files.length; index++) {
            const file = files[index];
            if (file && file.type.startsWith("image/")) {
                let { name, type } = file;
                let medium = {
                    name,
                    type,
                    url: URL.createObjectURL(file),
                    file,
                };
                media.push(medium);
            }
        }
        updateNewMedia(media);
    }

    function handleDrop(e) {
        e.preventDefault();
        addMedia(e.dataTransfer.files);
    }
</script>

<div {...rest}>
    <div
        class="full"
        ondrop={handleDrop}
        ondragover={preventDefault}
        role="listitem"
    >
        {#if Object.keys(offlineMedia).length === 0}
            <div
                class="flex full rounded-lg border-dashed border-2 border-gray-400"
            >
                <span class="block m-auto">Drop media here</span>
            </div>
        {:else}
            <ul class="full space-y-2">
                {#each Object.keys(offlineMedia)
                    .sort()
                    .map( (key) => ({ shortName: key, url: offlineMedia[key] }), ) as { shortName, url }, index (shortName)}
                    <MediumEntity
                        {shortName}
                        {url}
                        {changeName}
                        warning={shortName in onlineMedia &&
                            onlineMedia[shortName]}
                    />
                {/each}
            </ul>
        {/if}
    </div>
</div>
