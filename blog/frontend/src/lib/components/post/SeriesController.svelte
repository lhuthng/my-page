<script>
  import { auth } from "$lib/client/user";
  import { preventDefault } from "$lib/common";
  import PBody from "../PBody.svelte";

  let { series, series_slug } = $props();

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
  });
</script>

<div class="flex flex-col gap-2 bg-white rounded-sm p-2">
  {#if series && series.length > 0}
    <ul class="flex flex-col gap-2">
      {#each series as { title, slug }, index}
        <li class="bg-primary/20 hover:bg-primary/30 rounded-sm">
          <button class="px-2 py-1 w-full text-left">{title}</button>
        </li>
      {/each}
    </ul>
  {/if}
  <div class="ml-auto w-fit duo-btn duo-blue">
    <button onclick={() => (toggled = true)}>New Series</button>
  </div>
</div>

{#if toggled}
  <PBody>
    <div class="fixed z-5 flex w-screen h-screen items-center justify-center">
      <button
        class="absolute full cursor-default!"
        onclick={() => (toggled = false)}
        title="close-overlay"
      ></button>
      <div
        class="relative flex flex-col gap-4 z-10 bg-white p-2 rounded-xl text-dark text-lg"
      >
        <h2 class="mx-auto mb-2 text-xl font-semibold">New Series</h2>
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
                editor.coverError = "Only JPEG, PNG, GIF, or WEBP are allowed.";
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
        {#if editor.coverError}
          <span class="inline-block w-60 text-accent-red">
            *{editor.coverError}
          </span>
        {/if}
      </div>
    </div>
  </PBody>
{/if}
