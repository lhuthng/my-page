<script>
    import { auth } from "$lib/client/user";
    import { preventDefault } from "$lib/common";
    import MediaDirectory from "./MediaDirectory.svelte";
    import MediaEntity from "./MediaEntity.svelte";
    import MediaUploaderForm from "./MediaUploaderForm.svelte";
    import Portal from "./Portal.svelte";

    let { detailPanel, openDetails } = $props();

    let mediaList = $state([]);
    let selection = $state();
    let dragBox = $state();

    function appendMedia(files) {
        for (let index = 0; index < files.length; index++) {
            const file = files[index];
            if (file && file.type.startsWith("image/")) {
                let name = file.name;
                let media = {
                    name: name,
                    type: file.type,
                    url: URL.createObjectURL(file),
                    file,
                };
                mediaList.push(media);
            }
        }
    }

    function handleDrop(e) {
        e.preventDefault();
        appendMedia(e.dataTransfer.files);
    }
    function handleDragEnter(e) {
        if (e.target === e.currentTarget && dragBox) {
            dragBox.style.pointerEvents = "auto";
        }
    }
    function handleDragLeave(e) {
        if (e.target === e.currentTarget && dragBox) {
            dragBox.style.pointerEvents = "none";
        }
    }
</script>

<div
    class="relative full"
    ondragenter={handleDragEnter}
    ondragleave={handleDragLeave}
    role="listitem"
>
    {#if mediaList.length === 0}
        <div
            bind:this={dragBox}
            class="absolute full p-2 pointer-events-none"
            ondrop={handleDrop}
            ondragover={preventDefault}
            role="listitem"
        >
            <div
                class="flex full justify-center items-center border-2 border-dashed rounded-sm"
            >
                <p>Drop a media here</p>
            </div>
        </div>
    {/if}
    <MediaDirectory
        class="full p-2"
        cellWidth="120px"
        cellHeight="200px"
        ondrop={handleDrop}
        ondragover={preventDefault}
        onclick={(e) => {
            e.target === e.currentTarget && (selection = undefined);
        }}
    >
        {#each mediaList as media, index (media.name)}
            <MediaEntity
                size={80}
                file={media}
                onclick={() => (selection = index)}
                ondoubleclick={() => openDetails?.()}
                isSelected={index === selection}
                ok={media.ok}
            />
        {/each}
    </MediaDirectory>
</div>
<Portal class="p-2" target={detailPanel}>
    <MediaUploaderForm
        media={mediaList[selection]}
        onsuccess={() =>
            mediaList[selection] && (mediaList[selection].ok = true)}
        onfailed={() =>
            mediaList[selection] && (mediaList[selection].ok = false)}
    />
</Portal>
