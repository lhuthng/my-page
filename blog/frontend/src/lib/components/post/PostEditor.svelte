<script>
  import { auth, user } from "$lib/client/user";
  import {
    arraysEqualIgnoreOrder,
    nowToDate,
    preventDefault,
  } from "$lib/common";
  import { useDebounce } from "$lib/effects/debounce";
  import { fly } from "svelte/transition";
  import PostCard from "../home/PostCard.svelte";
  import ContentDebounceEdtior from "./ContentDebounceEdtior.svelte";
  import MediaDictionaryController from "./MediaDictionaryController.svelte";
  import PostSection from "./PostSection.svelte";
  import SeriesController from "./SeriesController.svelte";
  import PBody from "../PBody.svelte";

  const mediaSyntax = /\@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]/g;
  const allowedTypes = ["image/jpeg", "image/png", "image/gif", "image/webp"];
  const maxFileSize = 5 * 1024 * 1024;

  let { mode = "create", data } = $props();

  let mediaDictionary = $state({});
  let searchMedia = $state(async (keyword) => {});
  let forceContent = $state((content) => {});
  let isOnline = $state((keyword) => false);
  let isOffline = $state((keyword) => false);
  let getNewMedia = $state((keyword) => {});
  let clearNewMedia = $state((keywords) => {});

  const updateMediaDictionary = (newDictionary) => {
    mediaDictionary = { ...newDictionary };
  };

  let editingData = $state({
    id: "",
    title: "",
    _slugStatus: {},
    slug: "",
    tags: "",
    categories: [],
    series: [],
    seriesSlug: "",
    excerpt: "",
    date: nowToDate(),
    content: "",
    draft: "",
    coverUrl: "",
    author: {
      username: $user?.username,
      displayName: $user?.displayName,
      avatarUrl: $user?.avatarUrl,
    },
  });

  let editor = $state({
    coverToggled: false,
    toggled: false,
    view: "private",
    status: "",
    isCritical: false,
    isUploading: false,
    isUploaded: false,
    coverFile: undefined,
    newCover: undefined,
    coverError: "",
  });

  let renderedText = $state("");

  let forDraft = $derived(
    mode === "create" || (mode === "edit" && editor.view === "private"),
  );

  if (mode === "edit" && data !== undefined) {
    let {
      id,
      title,
      slug,
      series,
      seriesSlug,
      content,
      draft,
      excerpt,
      tags,
      coverUrl,
      mediumShortNames,
      mediumUrls,
    } = data;
    editingData.id = id;
    editingData.title = title;
    editingData.slug = slug;
    editingData.excerpt = excerpt;
    editingData.tags = tags.join(" ");
    editingData.content = content;
    editingData.draft = draft;
    editingData.series = series;
    editingData.seriesSlug = seriesSlug;
    editingData.coverUrl = coverUrl;

    editor.view = "public";
  }

  let slugDebounce = useDebounce(async (slug) => {
    if (slug.length < 5) return;

    if (!(slug in editingData._slugStatus)) {
      if (slug === data?.slug) {
        editingData._slugStatus[slug] = "ready";
      } else {
        editingData._slugStatus[slug] = "pending";
        const res = await fetch("/api/posts/check?slug=" + slug, {
          method: "GET",
          headers: {
            "Content-Type": "application/json",
          },
        });

        if (res.ok) {
          const { exists } = await res.json();
          editingData._slugStatus[slug] = !exists ? "ready" : "used";
        } else {
          delete editingData._slugStatus[slug];
        }
      }
    }
  }, 300);

  $effect(() => {
    forceContent(forDraft ? editingData.draft : editingData.content);
  });

  $effect(() => {
    slugDebounce.update(editingData.slug);
  });

  let timeout;
  $effect(() => {
    if (editor.status === "") return;

    editor.status;
    clearTimeout(timeout);
    timeout = setTimeout(() => {
      editor.status = "";
    }, 2000);
  });

  const newPost = async () => {
    let {
      title,
      slug,
      excerpt,
      tags,
      content = draft,
      categories,
    } = editingData;

    tags = tags
      .trim()
      .split(" ")
      .filter((tag) => tag !== "");

    const keys = [...editingData.content.matchAll(mediaSyntax)]
      .map((match) => match[1])
      .filter((key) => !isOnline(key));

    const missing = keys.filter((key) => !isOffline(key));

    if (missing.length > 0) {
      console.error("Missing keys detected: ", missing);
      return missing;
    }

    const formData = new FormData();
    formData.append(
      "post_data",
      new Blob(
        [
          JSON.stringify({
            title,
            slug,
            excerpt,
            tags,
            content,
            categories: [],
            number_of_files: keys.length,
          }),
        ],
        { type: "application/json" },
      ),
    );

    for (let index = 0; index < keys.length; index++) {
      const data = getNewMedia(keys[index]);
      formData.append(`file_${index + 1}`, data.file, data.name);
      formData.append(`short_name_${index + 1}`, keys[index]);
    }

    editor.isCritical = false;
    editor.status = "";

    const res = await fetch("/api/posts/new", {
      method: "POST",
      headers: { Authorization: auth() },
      body: formData,
    });

    if (res.ok) {
      // TODO: PREVENT RESUBMIT
      editor.isCritical = false;
      editor.status = "OK!";
    } else {
      editor.isCritical = true;
      editor.status = await res.text();
    }
  };

  const updatePost = async () => {
    const formData = new FormData();
    const postData = {
      number_of_files: 0,
    };
    let offlineKeys = [];
    if (editingData.draft !== data.draft) {
      const keys = [
        ...[
          ...editingData.content.matchAll(mediaSyntax).map((match) => match[1]),
        ],
        ...[
          ...editingData.draft.matchAll(mediaSyntax).map((match) => match[1]),
        ],
      ];

      offlineKeys = keys.filter((key) => !isOnline(key));

      const missing = offlineKeys.filter((key) => !isOffline(key));

      if (missing.length > 0) {
        editor.isCritical = true;
        editor.status = `[${missing}] is/are missing`;
        console.error("Missing keys detected: ", missing);
        return missing;
      }

      postData.number_of_files = offlineKeys.length;

      for (let index = 0; index < offlineKeys.length; index++) {
        const data = getNewMedia(offlineKeys[index]);
        formData.append(`file_${index + 1}`, data.file, data.name);
        formData.append(`short_name_${index + 1}`, offlineKeys[index]);
      }
    }

    if (editingData.title !== data.title) {
      postData.title = editingData.title;
    }

    if (editingData.slug !== data.slug) {
      postData.slug = editingData.slug;
    }

    const tags = editingData.tags
      .trim()
      .split(" ")
      .filter((tag) => tag !== "");
    if (!arraysEqualIgnoreOrder(editingData.tags, data.tags)) {
      postData.tags = tags;
    }

    if (editingData.excerpt !== data.excerpt) {
      postData.excerpt = editingData.excerpt;
    }

    if (editingData.draft !== data.draft) {
      postData.draft = editingData.draft;
      postData.content = editingData.content;
    }

    formData.append("post_data", new Blob([JSON.stringify(postData)]), {
      type: "application/json",
    });

    const res = await fetch("/api/posts/id/" + data.id, {
      method: "PATCH",
      headers: { Authorization: auth() },
      body: formData,
    });

    if (res.ok) {
      editor.isCritical = false;
      editor.status = "OK!";
      clearNewMedia(offlineKeys);
    } else {
      editor.isCritical = true;
      editor.status = await res.text();
    }
  };

  const publishPost = async () => {
    const res = await fetch("/api/posts/id/" + data.id, {
      method: "POST",
      headers: { Authorization: auth() },
    });

    if (res.ok) {
      // TODO: PREVENT RESUBMIT
      editor.isCritical = false;
      editor.status = "OK!";
    } else {
      editor.isCritical = true;
      editor.status = await res.text();
    }
  };
</script>

{#if editor.coverToggled}
  <PBody>
    <div
      class="sticky top-0 flex justify-center items-center w-full h-screen pointer-events-auto"
    >
      <div
        class="absolute cursor-not-allowed inset-0 z-10"
        onclick={() => {
          editor.coverToggled = false;
          editor.coverFile = undefined;
          editor.newCover = undefined;
          editor.isUploading = false;
          editor.isUploaded = false;
          editor.coverError = "";
        }}
        role="none"
      ></div>
      <div
        class="w-fit h-fit space-y-4 bg-white rounded-3xl p-4 text-xl z-11"
        role="none"
      >
        <div
          class="flex justify-center items-center w-60 h-60 bg-background/60 outline-4 outline-dark outline-dashed rounded-xl overflow-hidden"
          ondrop={(e) => {
            e.preventDefault();
            const file = e.dataTransfer.files[0];
            if (!file) {
              editor.coverError = "File not found!";
              return;
            }

            if (!allowedTypes.includes(file.type)) {
              editor.coverError = "Only JPEG, PNG, GIF, or WEBP are allowed.";
              return;
            }

            if (file.size > maxFileSize) {
              editor.coverError = `File size exceeds 5MB (${file.size} bytes)`;
              return;
            }

            editor.coverFile = file;
            editor.newCover = URL.createObjectURL(file);

            editor.isUploading = false;
            editor.isUploaded = false;
          }}
          ondragover={preventDefault}
          role="none"
        >
          {#if editor.newCover}
            <img
              class="full object-cover"
              src={editor.newCover}
              alt="temporal-avatar"
            />
          {:else}
            <span class="w-40 text-center select-none text-dark"
              >Upload your image here</span
            >
          {/if}
        </div>
        {#if editor.coverError}
          <span class="inline-block w-60 text-accent-red">
            *{editor.coverError}
          </span>
        {/if}
        <div class="duo-btn duo-green">
          <button
            disabled={!editor.newCover ||
              editor.isUploaded ||
              editor.isUploading}
            onclick={async () => {
              const formData = new FormData();

              formData.append("file", editor.coverFile, editor.coverFile.name);

              editor.isUploaded = false;
              editor.isUploading = true;
              const res = await fetch(`/api/posts/id/${editingData.id}/cover`, {
                method: "PATCH",
                headers: {
                  Authorization: auth(),
                },
                body: formData,
              });

              if (res.ok) {
                editor.isUploaded = true;
                editor.isUploading = false;
                editor.coverError = "";
                editingData.coverUrl = editor.newCover;
              } else {
                editor.isUploading = false;
                editor.coverError = await res.text();
              }
            }}>Apply</button
          >
        </div>
        {#if editor.isUploaded}
          <span class="inline-block w-60 text-accent-green">
            *New cover uploaded succesfully!
          </span>
        {/if}
      </div>
    </div>
  </PBody>
{/if}

<article class="relative flex flex-col gap-4 pb-4 *:drop-shadow-xl">
  <div class="flex w-full">
    <div class="p-2 rounded-xl bg-white mx-auto w-120">
      <PostCard
        id={editingData.id}
        dashboardMode={true}
        title={editingData.title === "" ? "<Empty>" : editingData.title}
        slug={editingData.slug}
        excerpt={editingData.excerpt === "" ? "<Empty>" : editingData.excerpt}
        author={{
          name: $user.displayName,
          slug: $user.username,
        }}
        tags={editingData.tags.split(" ").filter((tag) => tag !== "")}
        src={editingData.coverUrl}
        stats={{
          views: "#",
          likes: "#",
          comments_count: "#",
        }}
        onclick={() => {
          if (mode !== "edit") return;
          editor.coverToggled = true;
        }}
      >
        <div
          class="absolute top-0 full z-20 grid place-items-center border-4 border-dashed border-accent-green rounded-lg opacity-0 bg-accent-green/40 hover:opacity-100 hover:scale-105 transition-all duration-100"
        ></div>
      </PostCard>
    </div>
  </div>
  <PostSection
    title={editingData.title}
    tags={editingData.tags.split(" ").filter((tag) => tag !== "")}
    date={editingData.date}
    content={renderedText}
    author={editingData.author}
  />
  <div id="padding"></div>
  <div
    class="fixed top-full left-1/2 -translate-x-1/2 w-full max-w-400 transition-transform duration-100 -translate-y-14"
    class:-translate-y-full={editor.toggled}
  >
    <div
      class="absolute z-9 left-1/2 top-1/2 -translate-1/2 w-[calc(100%+6px)] h-[calc(100%+6px)] bg-dark/20 rounded-t-xl"
    ></div>
    <div
      class="relative z-10 flex flex-col items-center bg-white border-2 border-dark not-sm:text-sm rounded-t-xl"
    >
      <div class="flex justify-between p-2 w-full">
        <div class="flex gap-2">
          <div
            class="w-25 duo-btn"
            class:duo-green={!editor.toggled}
            class:duo-red={editor.toggled}
          >
            <button onclick={() => (editor.toggled = !editor.toggled)}
              >{editor.toggled ? "Collapse" : "Expand"}</button
            >
          </div>
          {#if mode === "edit"}
            <div in:fly={{ duration: 200 }} class="duo-btn duo-blue">
              <button
                onclick={() => {
                  switch (editor.view) {
                    case "public": {
                      forceContent(editingData.draft);
                      editor.view = "private";
                      editor.toggled = true;
                      return;
                    }
                    case "private": {
                      forceContent(editingData.content);
                      editor.view = "public";
                      return;
                    }
                  }
                }}
                >Ver. {editor.view === "public" ? "Published" : "Draft"}</button
              >
            </div>
          {/if}
        </div>
        <div class="flex gap-2">
          {#if editor.toggled}
            {#if mode === "create"}
              <div in:fly={{ duration: 200 }} class="duo-btn duo-green">
                <button onclick={newPost}>Submit</button>
              </div>
            {:else if mode === "edit"}
              <div
                class="my-auto"
                class:text-accent-green={!editor.isCritical}
                class:text-accent-red={editor.isCritical}
              >
                <span>{editor.status}</span>
              </div>
              <div in:fly={{ duration: 200 }} class="duo-btn duo-green">
                <button onclick={updatePost}>Change</button>
              </div>
              <div in:fly={{ duration: 200 }} class="duo-btn duo-green">
                <button onclick={publishPost}>Publish</button>
              </div>
            {/if}
          {/if}
        </div>
      </div>
      <div class="flex not-lg:flex-col gap-2 w-full h-full p-2 pt-1">
        <div class="flex grow gap-2">
          <div class="max-h-80 p-2 pr-1 w-1/3 bg-primary/40 rounded-lg">
            <div
              class="full space-y-2 pr-[3px] custom-scrollbar overflow-y-scroll"
            >
              <div class="flex not-sm:flex-col">
                <label class="inline-block min-w-11" for="title">Title: </label>
                <input
                  id="title"
                  class="grow px-1 min-w-0 bg-white rounded-sm"
                  bind:value={editingData.title}
                  autocomplete="off"
                  required
                />
              </div>
              <div class="flex not-sm:flex-col">
                <label class="inline-block min-w-11" for="slug">Slug: </label>
                <input
                  id="slug"
                  class="grow px-1 min-w-0 bg-white rounded-sm"
                  class:bg-red-200!={editingData._slugStatus[
                    editingData.slug
                  ] === "used"}
                  class:bg-yellow-200!={editingData._slugStatus[
                    editingData.slug
                  ] === "pending"}
                  class:bg-green-200!={editingData._slugStatus[
                    editingData.slug
                  ] === "ready"}
                  bind:value={editingData.slug}
                  autocomplete="off"
                  required
                />
              </div>
              <div class="flex flex-col">
                <label class="inline-block" for="slug">Tags: </label>
                <textarea
                  class="p-1 outline-none bg-white rounded-sm resize-none custom-scrollbar"
                  autocorrect="off"
                  autocomplete="off"
                  rows="2"
                  bind:value={editingData.tags}
                ></textarea>
              </div>
              <div class="flex flex-col">
                <label class="inline-block" for="slug">Excerpt: </label>
                <textarea
                  class="p-1 outline-none bg-white rounded-sm resize-none custom-scrollbar"
                  autocorrect="off"
                  autocomplete="off"
                  rows="5"
                  bind:value={editingData.excerpt}
                ></textarea>
              </div>
              {#if mode === "edit"}
                <SeriesController
                  postId={editingData.id}
                  series={editingData.series}
                  seriesSlug={editingData.seriesSlug}
                />
              {/if}
            </div>
          </div>
          <ContentDebounceEdtior
            class="max-h-80 grow bg-primary/40 p-2 rounded-lg"
            delay="500"
            onUpdateRendered={(_renderedText) => (renderedText = _renderedText)}
            onUpdateDraft={(content) => (editingData.draft = content)}
            disabled={editor.view === "public"}
            {mediaSyntax}
            {mediaDictionary}
            {searchMedia}
            {forDraft}
            registerForceContent={(fn) => {
              forceContent = fn;
            }}
          />
        </div>
        <MediaDictionaryController
          class="flex max-h-80 gap-2 not-lg:h-40 overflow-hidden"
          registerMediaCheck={({
            isOnline: _isOnline,
            isOffline: _isOffline,
          }) => {
            isOnline = _isOnline;
            isOffline = _isOffline;
          }}
          registerGetMedia={(fn) => (getNewMedia = fn)}
          {updateMediaDictionary}
          registerSearch={(fn) => (searchMedia = fn)}
          registerClearNewMedia={(fn) => (clearNewMedia = fn)}
        />
      </div>
    </div>
  </div>
</article>

<style lang="postcss">
  @reference "../../../app.css";

  #padding {
    @apply h-80;
  }
</style>
