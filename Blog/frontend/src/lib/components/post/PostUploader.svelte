<script>
    import { pick } from "$lib/client/misc";
    import { auth } from "$lib/client/user";
    import { useDebounce } from "$lib/effects/debounce";
    import { onMount } from "svelte";
    import MediaSearcher from "./MediaSearcher.svelte";
    import MediaUploader from "./MediaUploader.svelte";
    import PostRenderer from "./PostRenderer.svelte";

    const colorMap = {
        music: {
            bg: "#5A47C6",
            text: "#FFFFFF",
        },
        programming: {
            bg: "#008DB8",
            text: "#FFFFFF",
        },
        "digital-art": {
            bg: "#C43A75",
            text: "#FFFFFF",
        },
        game: {
            bg: "#00A77A",
            text: "#FFFFFF",
        },
        misc: {
            bg: "#546E7A",
            text: "#FFFFFF",
        },
        default: {
            bg: "#FFFFFF",
            text: "#000000",
        },
    };
    const getTextColor = (category) =>
        (colorMap[category] ?? colorMap.default).text;
    const getBackgroundColor = (category) =>
        (colorMap[category] ?? colorMap.default).bg;

    let draft = $state({
        content: {
            text: "",
            deText: "",
        },
        title: "",
        slug: {
            text: "",
            status: {},
        },
        categories: {
            all: [],
            status: "init",
        },
    });
    let slugDebounce = useDebounce(async (slug) => {
        if (!(slug in draft.slug.status)) {
            draft.slug.status[slug] = "pending";
            const res = await fetch(`/api/posts/check?slug=${slug}`, {
                method: "GET",
                headers: {
                    "Content-Type": "application/json",
                },
            });

            if (res.ok) {
                const { exists } = await res.json();
                draft.slug.status[slug] = !exists ? "available" : "occupied";
            } else {
                delete draft.slug.status[slug];
            }
        }
    }, 300);

    const placeholder = pick([
        [
            "The Mysterious Art of Doing Stuff™",
            "mysterious-art-of-doing-stuff",
        ],
        [
            "Why This Topic Is Definitely Important (Probably)",
            "topic-is-important-probably",
        ],
        [
            "Secrets No One Asked For but Here They Are",
            "secrets-nobody-asked-for",
        ],
        [
            "How to Be Amazing at Something You Haven’t Tried Yet",
            "be-amazing-at-anything",
        ],
        [
            "The Ultimate Guide to Whatever This Post Is About",
            "ultimate-guide-to-whatever",
        ],
    ]);

    $effect(() => {
        if (draft.slug.text.length >= 5) slugDebounce.update(draft.slug.text);
    });

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
    let mediaDictionary = $derived({
        ...offlineMedia,
        ...Object.fromEntries(
            Object.entries(onlineMedia).filter(([_, v]) => v !== undefined),
        ),
    });
    let markdownDebounce = useDebounce((text) => {
        draft.content.deText = text;
    }, 300);

    $effect(() => {
        markdownDebounce.update(draft.content.text);
    });

    onMount(async () => {
        const res = await fetch("/api/posts/categories", {
            method: "GET",
        });

        if (res.ok) {
            draft.categories.all = (await res.json()).categories.map(
                (category) => ({ ...category, selected: false }),
            );
            draft.categories.status = "successful";
        } else {
            draft.categories.status = "failed";
        }
    });

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

    const mediaSyntax = /\@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]/g;

    let textDebounce = useDebounce(async (text) => {
        const keys = [...text.matchAll(mediaSyntax)].map((match) => match[1]);
        findMedia(keys);
    }, 300);

    $effect(() => {
        textDebounce.update(draft.content.text);
    });

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
        if (!(newName in onlineMedia)) {
            findMedia([newName]);
        }
        return true;
    }

    async function post() {
        const {
            title,
            slug: { text: slug },
            content: { deText: content },
            categories,
        } = draft;

        const keys = [...content.matchAll(mediaSyntax)]
            .map((match) => match[1])
            .filter((key) => !onlineMedia[key]);

        const missingKeys = keys.filter((key) => !offlineMedia[key]);

        if (missingKeys.length > 0) return missingKeys;

        const formData = new FormData();

        const selectedCategories = categories.all
            .filter((c) => c.selected)
            .map((c) => c.slug);

        formData.append(
            "post_data",
            new Blob(
                [
                    JSON.stringify({
                        title,
                        slug,
                        content,
                        categories: selectedCategories,
                        number_of_files: keys.length,
                    }),
                ],
                { type: "application/json" },
            ),
        );

        for (let index = 0; index < keys.length; index++) {
            const data = offlineMediaData[keys[index]];
            formData.append(`file_${index + 1}`, data.file, data.name);
            formData.append(`short_name_${index + 1}`, keys[index]);
        }

        const res = await fetch("/api/posts/new", {
            method: "POST",
            headers: { Authorization: auth() },
            body: formData,
        });

        if (res.ok) {
            draft.slug.status[slug] = "occupied";
        }
    }

    async function publish() {
        const res = await fetch(`/api/posts/${draft.slug.text}`, {
            method: "PATCH",
            headers: {
                Authorization: auth(),
            },
        });
    }
</script>

<div class="py-2 space-y-2">
    <div class="space-y-2">
        <div>
            <label class="inline-block w-10" for="title">Title: </label>
            <input
                class={`w-80 border-b rounded-t-lg px-2 transition-colors duration-200 ${draft.title.length < 5 ? "bg-red-200" : "bg-green-200"}`}
                id="title"
                bind:value={draft.title}
                placeholder={placeholder[0]}
                required
            />
            <span class="text-sm text-gray-600 select-none">
                {#if draft.title.length < 5}
                    (min. 5 chars)
                {/if}
            </span>
        </div>
        <div>
            <label class="inline-block w-10" for="slug">Slug: </label>
            <input
                class={`w-80 border-b rounded-t-lg px-2 transition-colors duration-200 ${draft.slug.text.length < 5 || draft.slug.status[draft.slug.text] === "occupied" ? "bg-red-200" : draft.slug.status[draft.slug.text] === undefined || draft.slug.status[draft.slug.text] === "pending" ? "bg-yellow-200" : "bg-green-200"}`}
                id="slug"
                bind:value={
                    () => draft.slug.text,
                    (value) => {
                        draft.slug.text = value;
                    }
                }
                placeholder={placeholder[1]}
                required
            />
            <span class="text-sm text-gray-600 select-none">
                {#if draft.slug.text.length < 5}
                    (min. 5 chars)
                {:else if draft.slug.status[draft.slug.text] === "occupied"}
                    ("{draft.slug.text}" is already used)
                {:else if draft.slug.status[draft.slug.text] === "available"}
                    (sounds good)
                {/if}
            </span>
        </div>
        <div class="flex space-x-2">
            <label class="block w-20" for="categories">Categories: </label>
            <div id="categories" class="px-2">
                {#if draft.categories.status === "successful"}
                    <ol
                        class="flex flex-wrap items-center gap-x-2 gap-y-1 w-65"
                    >
                        {#each draft.categories.all.toSorted() as category, index (category.slug)}
                            <li>
                                <button
                                    id={`${category.slug}-btn`}
                                    class="inline-block h-fit origin-center rounded-full px-2 text-sm cursor-pointer border hover:brightness-120 transition-all duration-200"
                                    style:border-color={category.selected
                                        ? getBackgroundColor(category.slug)
                                        : "black"}
                                    style:color={category.selected
                                        ? getTextColor(category.slug)
                                        : "black"}
                                    style:background-color={category.selected
                                        ? getBackgroundColor(category.slug)
                                        : "unset"}
                                    onclick={() =>
                                        (draft.categories.all[index].selected =
                                            !draft.categories.all[index]
                                                .selected)}
                                >
                                    {category.slug}
                                </button>
                            </li>
                        {/each}
                    </ol>
                {:else}
                    <span>Loading...</span>
                {/if}
            </div>
        </div>
    </div>

    <div class="flex gap-2">
        <div
            class="flex flex-col justify-between min-h-180 grow border border-gray-300 rounded-lg overflow-hidden"
        >
            <div class="p-2 grow">
                <PostRenderer text={draft.content.deText} {mediaDictionary} />
            </div>
            <div class="h-40 border-t border-t-gray-300 bg-gray-200">
                <textarea
                    class="w-full h-full p-2 resize-none"
                    placeholder="Type here..."
                    bind:value={draft.content.text}
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
                    class="w-20 text-center rounded-xl border border-gray-300 not-disabled:cursor-pointer not-disabled:hover:bg-gray-200 text-gray-600 disabled:text-gray-300 px-2 transition-colors duration-200"
                    onclick={post}
                    disabled={draft.slug.text.length < 5 ||
                        draft.slug.status[draft.slug.text] !== "available"}
                >
                    Save
                </button>
                <button
                    class="w-20 text-center rounded-xl border border-gray-300 text-gray-600 not-disabled:cursor-pointer not-disabled:hover:bg-gray-200 disabled:text-gray-300 px-2 transition-colors duration-200"
                    onclick={publish}
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
