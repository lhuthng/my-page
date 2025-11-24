<script>
    import MediaDirectory from "./MediaDirectory.svelte";
    import MediaEditor from "./MediaEditor.svelte";
    import MediaUploader from "./MediaUploader.svelte";

    let { editMode, changeMode, ...rest } = $props();
    let keyword = $state("");
    let tab = $state(0);
    let detailPanel = $state();
</script>

<div {...rest}>
    <div class="flex flex-col px-2 not-md:pr-6 py-4 gap-4">
        <div class="flex justify-between">
            <div class="flex gap-2">
                <input
                    disabled={editMode !== true}
                    class="rounded-xl ring-1 px-2 disabled:bg-gray-100 disabled:text-gray-400 disabled:cursor-not-allowed"
                    type="text"
                    placeholder="keywords"
                    bind:value={keyword}
                />
                <button
                    disabled={editMode !== true}
                    class="border-black rounded-full px-2 ring-1 hover:bg-gray-50 disabled:bg-gray-100 disabled:text-gray-400 disabled:cursor-not-allowed"
                    type="button">Search</button
                >
            </div>

            <button
                class="cursor-pointer"
                type="button"
                onclick={() => changeMode?.()}
                >{editMode ? "Upload" : "Edit"}</button
            >
        </div>
        <div class="relative z-9">
            <div
                class="relative flex rounded-lg ring-1 h-180 overflow-hidden z-10"
            >
                <div class="grow bg-blue-100 min-h-full">
                    {#if editMode}
                        <MediaEditor
                            {detailPanel}
                            {keyword}
                            openDetails={() => (tab = 1)}
                        />
                    {:else}
                        <MediaUploader
                            {detailPanel}
                            openDetails={() => (tab = 1)}
                        ></MediaUploader>/>
                    {/if}
                </div>
                <div
                    class="relative h-full bg-white transition-all duration-200"
                    style:width={tab === 1 ? "min(20rem,50%)" : "0"}
                >
                    <div class="absolute w-80 h-full">
                        <div bind:this={detailPanel}></div>
                    </div>
                </div>
            </div>
            <div class="absolute top-2 left-full z-9">
                <button
                    class={`bg-blue-100 rounded-r-lg ring-1 px-2 pl-1 cursor-pointer transition-all duration-200 ${tab === 1 ? "translate-x-0 bg-blue-300" : "-translate-x-1 hover:-translate-x-0.5 hover:bg-blue-200"}`}
                    style:writing-mode="vertical-lr"
                    onclick={() => (tab = tab === 1 ? 0 : 1)}>Details</button
                >
            </div>
        </div>
    </div>
</div>
