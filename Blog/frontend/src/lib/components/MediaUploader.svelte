<script>
    import { getAuthorization } from "$lib/client/user";
    import { preventDefault } from "$lib/common";
    import MediaDirectory from "./MediaDirectory.svelte";
    import MediaEntity from "./MediaEntity.svelte";
    import Portal from "./Portal.svelte";

    let { detailPanel, openDetails } = $props();

    let mediaList = $state([]);
    let hasMedia = $derived(mediaList.length > 0);
    let selection = $state(undefined);
    let dragBox = $state(undefined);
    let shortName = $state("");
    let description = $state("");

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
    async function handleSubmit(e) {
        e.preventDefault();

        if (
            selection === undefined ||
            mediaList.length < selection ||
            selection < 0
        ) {
            return;
        }

        const formData = new FormData();
        const { file, name } = mediaList[selection];

        formData.append("file", file, name);
        formData.append("short_name", shortName);
        formData.append("description", description);

        try {
            const authorization = getAuthorization();
            const res = await fetch("/api/media", {
                method: "POST",
                headers: { Authorization: authorization },
                body: formData,
            });
        } catch (e) {
            console.error(e);
        }
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
    {#if !hasMedia}
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
            />
        {/each}
    </MediaDirectory>
</div>
<Portal class="p-2" target={detailPanel}>
    {#if !hasMedia}
        <div>Upload and select any media to prepare the file.</div>
    {:else}
        <div>
            {#if selection !== undefined}
                <form
                    method="post"
                    enctype="multipart/form-data"
                    onsubmit={handleSubmit}
                >
                    <label for="filename">
                        Filename: {mediaList[selection].name}<br />
                    </label>
                    <label for="content-type">
                        Content type: {mediaList[selection].type}<br />
                    </label>
                    <label>
                        Short name:
                        <input
                            class="border"
                            type="text"
                            name="short_name"
                            bind:value={shortName}
                            required
                        />
                    </label>
                    <label>
                        Description:
                        <input
                            class="border"
                            type="text"
                            name="description"
                            bind:value={description}
                            required
                        />
                    </label>
                    <button type="submit">Submit</button>
                </form>
            {/if}
        </div>
    {/if}
</Portal>
