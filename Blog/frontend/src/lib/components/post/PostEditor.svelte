<script>
    import { user } from "$lib/client/user";
    import { nowToDate } from "$lib/common";
    import ContentDebounceEdtior from "./ContentDebounceEdtior.svelte";
    import MediaDictionaryController from "./MediaDictionaryController.svelte";
    import PostSection from "./PostSection.svelte";

    let mediaDictionary = $state({});
    let searchMedia = $state(() => {});

    const updateMediaDictionary = (newDictionary) => {
        mediaDictionary = { ...newDictionary };
    };

    let draft = $state({
        title: "",
        slug: "",
        tags: [],
        date: nowToDate(),
        _content: "",
        content: "",
        author: {
            username: $user.username,
            displayName: $user.displayName,
            avatarUrl: $user.avatarUrl,
        },
    });

    let editor = $state({
        toggled: true,
    });

    const post = async () => {};
</script>

<article class="relative flex flex-col gap-4 pb-4 *:drop-shadow-xl">
    <PostSection
        title={draft.title}
        tags={draft.tags}
        date={draft.date}
        content={draft.content}
        author={draft.author}
    />
    <div
        class="fixed top-full left-1/2 -translate-x-1/2 w-cap flex flex-col items-center transition-transform duration-100 translate-y-[calc(-100%)]"
    >
        <button
            class="z-9 toggle-btn absolute left-1/2 -translate-x-1/2 w-fit h-auto p-1 pr-2 bg-white rounded-t-2xl -translate-y-8 hover:-translate-y-9 transition-transform duration-100 border-2 border-b-0 cursor-pointer"
            onclick={() => (editor.toggled = !editor.toggled)}
        >
            <svg
                class="inline w-6 h-6 fill-dark"
                class:-scale-y-100={!editor.toggled}
                viewBox="0 0 24 24"
            >
                <path
                    fill-rule="evenodd"
                    clip-rule="evenodd"
                    d="M3 12C3 7.02944 7.02944 3 12 3C16.9706 3 21 7.02944 21 12C21 16.9706 16.9706 21 12 21C7.02944 21 3 16.9706 3 12ZM12 1C5.92487 1 1 5.92487 1 12C1 18.0751 5.92487 23 12 23C18.0751 23 23 18.0751 23 12C23 5.92487 18.0751 1 12 1ZM7.70711 9.29289C7.31658 8.90237 6.68342 8.90237 6.29289 9.29289C5.90237 9.68342 5.90237 10.3166 6.29289 10.7071L11.2929 15.7071C11.6834 16.0976 12.3166 16.0976 12.7071 15.7071L17.7071 10.7071C18.0976 10.3166 18.0976 9.68342 17.7071 9.29289C17.3166 8.90237 16.6834 8.90237 16.2929 9.29289L12 13.5858L7.70711 9.29289Z"
                >
                </path>
            </svg>
            <span>edit</span>
        </button>
        <div
            class="relative z-10 flex not-lg:flex-col gap-2 w-full max-h-100 h-100 bg-white p-2 outline-2 rounded-t-xl"
        >
            <div class="flex grow gap-2">
                <div class="p-2 bg-primary/40 rounded-lg">
                    <div>
                        <label class="inline-block w-11" for="title"
                            >Title:
                        </label>
                        <input
                            id="title"
                            class=""
                            bind:value={draft.title}
                            autocomplete="off"
                            required
                        />
                    </div>
                    <div>
                        <label class="inline-block w-11" for="slug"
                            >Slug:
                        </label>
                        <input
                            id="slug"
                            class=""
                            bind:value={draft.slug}
                            autocomplete="off"
                            required
                        />
                    </div>
                </div>
                <ContentDebounceEdtior
                    class="grow bg-primary/40 rounded-lg"
                    delay="500"
                    onupdate={(content) => (draft.content = content)}
                    {mediaDictionary}
                    {searchMedia}
                />
            </div>
            <div class="flex gap-2 not-lg:h-60 overflow-hidden">
                <MediaDictionaryController
                    {updateMediaDictionary}
                    registerSearch={(fn) => (searchMedia = fn)}
                />
            </div>
        </div>
    </div>
</article>

<style lang="postcss">
    @reference "../../../app.css";

    .toggle-btn {
        &::after {
            @apply absolute content-[''] top-full left-0 w-full h-1 bg-white;
        }
    }
</style>
