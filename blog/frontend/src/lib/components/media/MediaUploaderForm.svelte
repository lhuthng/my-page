<script>
    import { auth } from "$lib/client/user";

    let { media, onsuccess, onfailed } = $props();

    let draft = $state({});

    $effect(() => {
        if (media !== undefined) {
            const { file, name, description, type } = media;
            draft = {
                shortName: "",
                file,
                name,
                description,
                type,
            };
        }
    });
</script>

<div class="flex flex-col gap-2">
    {#if !media}
        <span>?</span>
    {:else}
        <span class="text-center">Details</span>
        <form
            class="flex flex-col gap-2"
            method="post"
            onsubmit={async (e) => {
                e.preventDefault();

                if (!media) return;

                const formData = new FormData();

                formData.append("file", draft.file, draft.name);
                formData.append("short_name", draft.shortName);
                formData.append("description", draft.description);

                const res = await fetch("/api/media/upload", {
                    method: "POST",
                    headers: {
                        Authorization: auth(),
                    },
                    body: formData,
                });

                if (res.ok) {
                    onsuccess?.();
                } else {
                    onfailed?.();
                }
            }}
        >
            <label class="ml-auto mr-4 text-sm" for="file-type">
                {draft.type}
            </label>
            <fieldset class="border-2 rounded-lg pt-1 pb-2 px-2">
                <legend
                    class="font-semibold text-xs left-2 px-1"
                    for="short-name"
                >
                    Short name
                </legend>
                <input
                    disabled={media?.ok}
                    class="w-full focus:outline-0"
                    type="text"
                    bind:value={draft.shortName}
                    name="short-name"
                />
            </fieldset>
            <fieldset class="border-2 rounded-lg pt-1 pb-2 px-2">
                <legend
                    class="font-semibold text-xs left-2 px-1"
                    for="description"
                >
                    Description
                </legend>
                <textarea
                    disabled={media?.ok}
                    class="w-full focus:outline-0 resize-none"
                    type="text"
                    rows="4"
                    bind:value={draft.description}
                    name="short-name"
                ></textarea>
            </fieldset>
            <button
                disabled={media?.ok}
                class="ml-auto w-fit border-2 px-1 rounded-lg cursor-pointer"
                type="submit"
            >
                Submit
            </button>
        </form>
    {/if}
</div>
