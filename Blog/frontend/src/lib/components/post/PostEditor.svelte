<script>
    import MediaSearcher from "./MediaSearcher.svelte";
    import MediaUploader from "./MediaUploader.svelte";
    import PostRender from "./PostRender.svelte";

    let text = $state("");
    const mediaSyntax = /\@(?:\([\d_]+\))?\[\w+:([^\]]+)\]/g;

    let onlineMedia = $state({});
    let offlineMediaData = $state({});
    let offlineMedia = $derived(
        Object.fromEntries(
            Object.entries(offlineMediaData).map(([key, value]) => [
                key,
                value.url,
            ]),
        ),
    );
    let mediaDictionary = $derived({ ...offlineMedia, ...onlineMedia });

    async function findMedia(keys) {
        const temp = { ...onlineMedia };
        const missingKeys = keys.filter((key) => !(key in onlineMedia));

        missingKeys.forEach((key) => (temp[key] = null));
        onlineMedia = { ...temp };

        missingKeys.forEach(async (key) => {
            try {
                const res = await fetch("/api/media/s/" + key, {
                    method: "GET",
                });
                if (res.ok) {
                    onlineMedia[key] = (await res.json()).url;
                } else {
                    onlineMedia[key] = undefined;
                    setTimeout(() => delete onlineMedia[key], 3000);
                }
            } catch (e) {}
        });
    }

    function updateOfflineMedia(media) {
        const temp = { ...offlineMediaData };
        const names = [];
        media.forEach((medium) => {
            temp[medium.name] = medium;
            names.push(medium.name);
        });
        findMedia(names);
        offlineMediaData = { ...temp };
    }

    function changeName(oldName, newName) {
        if (newName in offlineMediaData) return false;

        let temp = { ...offlineMediaData };
        temp[newName] = temp[oldName];

        delete temp[oldName];

        offlineMediaData = { ...temp };
        return true;
    }

    function post() {}
</script>

<div class="py-2 space-y-2">
    <div>
        <div>
            <label for="title">Title: </label>
            <input class="border-b" id="title" required />
        </div>
        <div>
            <label for="slug">Slug: </label>
            <input class="border-b" id="slug" required />
        </div>
    </div>

    <div class="flex gap-2">
        <div
            class="flex flex-col justify-between min-h-180 grow border border-gray-300 rounded-lg overflow-hidden"
        >
            <div class="p-2 grow">
                <PostRender
                    {text}
                    {mediaDictionary}
                    {mediaSyntax}
                    handlers={{ findMedia }}
                />
            </div>
            <div class="h-40 border-t border-t-gray-300 bg-gray-200">
                <textarea
                    class="w-full h-full p-2 resize-none"
                    placeholder="Type here..."
                    bind:value={text}
                ></textarea>
            </div>
        </div>
        <div class="w-1/3 min-w-80 max-w-100 space-y-4">
            <div
                class="w-full h-fit bg-gray-200 border border-gray-300 rounded-lg space-y-2"
            >
                <MediaSearcher
                    class="flex flex-col h-80 border-b border-gray-300 overflow-hidden"
                />
                <MediaUploader
                    class="h-80 border-t border-gray-300 p-2"
                    {offlineMedia}
                    {onlineMedia}
                    handlers={{
                        updateOfflineMedia,
                        changeName,
                    }}
                />
            </div>
            <div class="flex justify-evenly">
                <button
                    class="w-20 text-center rounded-xl border border-gray-300 cursor-pointer hover:bg-gray-200 px-2 transition-colors duration-200"
                    onclick={post}
                >
                    Save
                </button>
                <button
                    class="w-20 text-center rounded-xl border border-gray-300 cursor-pointer hover:bg-gray-200 px-2 transition-colors duration-200"
                >
                    Publish
                </button>
            </div>
        </div>
    </div>
</div>

<style lang="postcss">
    @reference "tailwindcss";

    input,
    textarea {
        @apply focus:outline-0;
    }
</style>
