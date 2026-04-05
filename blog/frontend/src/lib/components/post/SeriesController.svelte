<script>
  import { auth } from "$lib/client/user";
  import PBody from "../PBody.svelte";

  let {
    postId = null,
    series = $bindable([]),
    seriesSlug = $bindable(""),
    onSelect = null,
  } = $props();

  const maxFileSize = 5 * 1024 * 1024;
  const allowedTypes = ["image/jpeg", "image/png", "image/gif", "image/webp"];
  const clientUrl = (url) =>
    url ? `/api/${url}`.replace("/./", "/") : undefined;

  let toggled = $state(false);
  let editor = $state({
    title: "",
    slug: "",
    description: "",
    coverImage: undefined,
    coverFile: undefined,
    coverError: "",
    isUploading: false,
    isUploaded: false,
    seriesMode: "",
    selected: null,
  });

  let selectedForCreate = $state(null);

  const closeModal = () => {
    toggled = false;
    editor.seriesMode = "";
    editor.selected = null;
    editor.coverError = "";
  };

  const handleAddToSeries = async () => {
    const { id, slug } = editor.selected;

    if (!postId) {
      onSelect?.(id);
      selectedForCreate = editor.selected;
      closeModal();
      return;
    }

    const res = await fetch(`/api/series/id/${id}?post_id=${postId}`, {
      method: "PATCH",
      headers: { Authorization: auth() },
    });

    if (res.ok) {
      seriesSlug = slug;
      closeModal();
    } else {
      editor.coverError = await res.text();
    }
  };

  const handleNewSeries = async () => {
    const formData = new FormData();
    if (editor.coverFile) {
      formData.append("file", editor.coverFile, editor.coverFile.name);
    }
    formData.append("title", editor.title);
    formData.append("slug", editor.slug);
    formData.append("description", editor.description);

    editor.coverError = "";
    editor.isUploaded = false;
    editor.isUploading = true;

    const res = await fetch("/api/series/new", {
      method: "POST",
      headers: { Authorization: auth() },
      body: formData,
    });

    if (res.ok) {
      const listRes = await fetch("/api/series/all", {
        headers: { Authorization: auth() },
      });
      if (listRes.ok) {
        const { series: fresh } = await listRes.json();
        series = fresh.map((s) => ({ ...s, url: clientUrl(s.url) }));
      }
      editor.isUploaded = true;
      editor.isUploading = false;
      editor.title = "";
      editor.slug = "";
      editor.description = "";
      editor.coverImage = undefined;
      editor.coverFile = undefined;
    } else {
      editor.isUploading = false;
      editor.coverError = await res.text();
    }
  };
</script>

<div class="flex flex-col gap-2 bg-white rounded-sm p-2">
  {#if series && series.length > 0}
    <ul class="flex flex-col gap-2">
      {#each series as item}
        <li class="bg-primary/20 hover:bg-primary/30 rounded-sm">
          <button
            class="px-2 py-1 w-full text-left"
            onclick={() => {
              editor.seriesMode = "add";
              editor.selected = item;
              editor.coverError = "";
              toggled = true;
            }}
          >
            {item.title}
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if !postId && selectedForCreate}
    <div
      class="flex items-center gap-2 text-sm bg-accent-green/10 px-2 py-1 rounded-sm"
    >
      <span>→ <strong>{selectedForCreate.title}</strong></span>
      <button
        class="ml-auto text-accent-red"
        onclick={() => {
          onSelect?.(null);
          selectedForCreate = null;
        }}>✕</button
      >
    </div>
  {/if}

  {#if postId && seriesSlug}
    <div class="text-sm text-primary/60 px-2">
      Attached to: <i>{seriesSlug}</i>
    </div>
  {/if}

  <div class="ml-auto w-fit duo-btn duo-blue">
    <button
      onclick={() => {
        editor.seriesMode = "new";
        editor.coverError = "";
        editor.isUploaded = false;
        toggled = true;
      }}>New Series</button
    >
  </div>
</div>

{#if toggled}
  <PBody>
    <div class="fixed z-5 flex w-screen h-screen items-center justify-center">
      <button
        class="absolute full cursor-default!"
        onclick={closeModal}
        title="close-overlay"
      ></button>
      <div
        class="relative flex flex-col gap-4 z-10 bg-white p-4 rounded-xl text-dark text-lg max-w-xl w-full mx-4"
      >
        <h2 class="mx-auto mb-2 text-xl font-semibold">
          {editor.seriesMode === "new" ? "New Series" : "Add to Series"}
        </h2>

        {#if editor.seriesMode === "new"}
          <div class="flex not-lg:flex-col gap-2">
            <div class="flex flex-col gap-1">
              <div class="grid grid-cols-[5rem_auto] gap-1 items-center">
                <span>Title:</span>
                <input
                  class="bg-primary/40 px-2 py-0.5 rounded-sm"
                  type="text"
                  bind:value={editor.title}
                />
                <span>Slug:</span>
                <input
                  class="bg-primary/40 px-2 py-0.5 rounded-sm"
                  type="text"
                  bind:value={editor.slug}
                />
              </div>
              <span>Description:</span>
              <textarea
                class="bg-primary/40 p-2 rounded-sm resize-none outline-none custom-scrollbar"
                autocorrect="off"
                autocomplete="off"
                rows="4"
                bind:value={editor.description}
              ></textarea>
            </div>
            <div
              class="flex shrink-0 m-auto justify-center items-center w-40 h-40 bg-primary/40 outline-4 outline-dark outline-dashed rounded-xl overflow-hidden"
              role="none"
              ondrop={(e) => {
                e.preventDefault();
                const file = e.dataTransfer.files[0];
                if (!file) {
                  editor.coverError = "File not found!";
                  return;
                }
                if (!allowedTypes.includes(file.type)) {
                  editor.coverError =
                    "Only JPEG, PNG, GIF, or WEBP are allowed.";
                  return;
                }
                if (file.size > maxFileSize) {
                  editor.coverError = `File size exceeds 5MB`;
                  return;
                }
                editor.coverFile = file;
                editor.coverImage = URL.createObjectURL(file);
                editor.isUploading = false;
                editor.isUploaded = false;
              }}
              ondragover={(e) => e.preventDefault()}
            >
              {#if editor.coverImage}
                <img
                  class="full object-cover"
                  src={editor.coverImage}
                  alt="series cover"
                />
              {:else}
                <span class="w-32 text-center select-none text-dark text-base"
                  >Drop cover here</span
                >
              {/if}
            </div>
          </div>

          {#if editor.isUploaded}
            <span class="text-accent-green text-base">✓ Series created!</span>
          {/if}

          <div class="mx-auto w-fit duo-btn duo-green">
            <button
              disabled={editor.isUploading || !editor.title || !editor.slug}
              onclick={handleNewSeries}>Submit</button
            >
          </div>
        {:else if editor.selected}
          {@const { title, slug, description, url } = editor.selected}
          {#if url}
            <img
              class="mx-auto w-40 h-40 rounded-sm object-cover"
              src={url}
              alt="series cover"
            />
          {/if}
          <ul class="list-disc list-inside">
            <li><strong>Title:</strong> {title}</li>
            <li><strong>Slug:</strong> {slug}</li>
            {#if description}<li>
                <strong>Description:</strong>
                {description}
              </li>{/if}
          </ul>

          {#if postId && seriesSlug}
            <p>This post is attached to <i>{seriesSlug}</i>.</p>
          {:else if !postId && selectedForCreate?.id === editor.selected.id}
            <p class="text-accent-green">✓ Selected for this post</p>
            <div class="flex justify-evenly">
              <div class="duo-btn duo-red">
                <button
                  onclick={() => {
                    onSelect?.(null);
                    selectedForCreate = null;
                    closeModal();
                  }}
                >
                  Deselect
                </button>
              </div>
            </div>
          {:else}
            <p>
              <i
                >{postId
                  ? "Add this post to this series?"
                  : "Assign this series after creation?"}</i
              >
            </p>
            <div class="flex justify-evenly">
              <div class="duo-btn duo-green">
                <button onclick={handleAddToSeries}>Yes</button>
              </div>
            </div>
          {/if}
        {/if}

        {#if editor.coverError}
          <span class="text-accent-red text-base">*{editor.coverError}</span>
        {/if}
      </div>
    </div>
  </PBody>
{/if}
