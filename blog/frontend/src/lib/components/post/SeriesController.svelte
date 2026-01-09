<script>
  import { auth } from "$lib/client/user";
  import { preventDefault } from "$lib/common";
  import PBody from "../PBody.svelte";

  let { postId, series, seriesSlug } = $props();

  const maxFileSize = 5 * 1024 * 1024;
  const allowedTypes = ["image/jpeg", "image/png", "image/gif", "image/webp"];

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
  });
</script>

<div class="flex flex-col gap-2 bg-white rounded-sm p-2">
  {#if series && series.length > 0}
    <ul class="flex flex-col gap-2">
      {#each series as { title }, index}
        <li class="bg-primary/20 hover:bg-primary/30 rounded-sm">
          <button
            class="px-2 py-1 w-full text-left"
            onclick={() => {
              editor.seriesMode = "add";
              editor.selected = series[index];
              toggled = true;
            }}
          >
            {title}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
  <div class="ml-auto w-fit duo-btn duo-blue">
    <button
      onclick={() => {
        editor.seriesMode = "new";
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
        onclick={() => {
          editor.seriesMode = "";
          toggled = false;
        }}
        title="close-overlay"
      ></button>
      <div
        class="relative flex flex-col gap-4 z-10 bg-white p-2 rounded-xl text-dark text-lg"
      >
        <h2 class="mx-auto mb-2 text-xl font-semibold">
          {#if editor.seriesMode === "new"}
            New Series
          {:else}
            Edit Series
          {/if}
        </h2>
        {#if editor.seriesMode === "new"}
          <div class="flex not-lg:flex-col gap-2">
            <div class="flex flex-col">
              <div class="grid grid-cols-[3rem_auto]">
                <span>Title:</span>
                <input
                  class="bg-primary/40 px-2 mb-1 rounded-sm"
                  type="text"
                  bind:value={editor.title}
                />
                <span>Slug:</span>
                <input
                  class="bg-primary/40 px-2 mb-1 rounded-sm"
                  type="text"
                  bind:value={editor.slug}
                />
              </div>
              <span>Description:</span>
              <textarea
                class="bg-primary/40 p-2 mb-2 rounded-sm resize-none outline-none custom-scrollbar"
                autocorrect="off"
                autocomplete="off"
                bind:value={editor.description}
              ></textarea>
            </div>
            <div
              class="flex m-auto justify-center items-center w-60 h-60 bg-primary/40 outline-4 outline-dark outline-dashed rounded-xl overflow-hidden"
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
                  editor.coverError = `File size exceeds 5MB (${file.size} bytes)`;
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
                  alt="temporal-cover"
                />
              {:else}
                <span class="w-40 text-center select-none text-dark"
                  >Upload your image here</span
                >
              {/if}
            </div>
          </div>
          <div class="mx-auto w-fit duo-btn duo-green">
            <button
              disabled={editor.isUploading}
              onclick={async () => {
                const formData = new FormData();

                if (editor.coverFile) {
                  formData.append(
                    "file",
                    editor.coverFile,
                    editor.coverFile.name,
                  );
                }

                formData.append("title", editor.title);
                formData.append("slug", editor.slug);
                formData.append("description", editor.description);

                editor.isUploaded = false;
                editor.isUploading = true;
                const res = await fetch("/api/series/new", {
                  method: "POST",
                  headers: {
                    Authorization: auth(),
                  },
                  body: formData,
                });

                if (res.ok) {
                  editor.isUploaded = true;
                  editor.isUploading = false;
                  editor.coverError = "";
                } else {
                  editor.isUploading = false;
                  editor.coverError = await res.text();
                }
              }}
            >
              Submit
            </button>
          </div>
        {:else if editor.selected}
          {@const { id, title, slug, description, url } = editor.selected}
          {#if url}
            <img class="mx-auto w-40 h-40 rounded-sm" src={url} alt="series" />
          {/if}
          <ul class="list-disc list-inside max-w-120">
            <li><span><strong>Title:</strong> {title}</span></li>
            <li><span><strong>Slug:</strong> {slug}</span></li>
            <li><span><strong>Description:</strong> {description}</span></li>
          </ul>
          {#if seriesSlug}
            <div>
              <span>This post has been attached to <i>{seriesSlug}</i></span>
            </div>
          {:else}
            <div>
              <span><i>Do you want to add this post to this series?</i></span>
            </div>
            <div class="flex justify-between">
              <div class="duo-btn duo-green">
                <button
                  onclick={async () => {
                    const res = await fetch(
                      `/api/series/id/${id}?post_id=${postId}`,
                      {
                        method: "PATCH",
                        headers: {
                          Authorization: auth(),
                        },
                      },
                    );

                    console.log(res.ok);
                  }}
                >
                  Yes
                </button>
              </div>
              <div class="duo-btn duo-blue">
                <button disabled>Yes, with a number</button>
              </div>
            </div>
          {/if}
        {/if}
        {#if editor.coverError}
          <span class="inline-block w-60 text-accent-red">
            *{editor.coverError}
          </span>
        {/if}
      </div>
    </div>
  </PBody>
{/if}
