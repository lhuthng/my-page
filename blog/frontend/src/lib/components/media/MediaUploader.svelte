<script>
  import { auth } from "$lib/client/user";
  import { preventDefault } from "$lib/common";
  import MediaDirectory from "./MediaDirectory.svelte";
  import MediaUploaderForm from "./MediaUploaderForm.svelte";
  import Portal from "$lib/components/Portal.svelte";
  import MediumEntity from "./MediumEntity.svelte";

  let { detailPanel, openDetails } = $props();

  let media = $state([]);
  let selection = $state();
  let dragBox = $state();

  function appendMedia(files) {
    for (let index = 0; index < files.length; index++) {
      const file = files[index];
      if (file && file.type.startsWith("image/")) {
        let name = file.name;
        let medium = {
          name: name,
          type: file.type,
          url: URL.createObjectURL(file),
          file,
        };
        media.push(medium);
      }
    }
  }

  function handleDrop(e) {
    e.preventDefault();
    appendMedia(e.dataTransfer.files);
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
  {#if media.length === 0}
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
    {#each media as medium, index (medium.name)}
      <MediumEntity
        size={80}
        file={medium}
        onclick={() => (selection = index)}
        ondoubleclick={() => openDetails?.()}
        isSelected={index === selection}
        ok={medium.ok}
      />
    {/each}
  </MediaDirectory>
</div>
<Portal class="p-2" target={detailPanel}>
  <MediaUploaderForm
    media={media[selection]}
    onsuccess={() => media[selection] && (media[selection].ok = true)}
    onfailed={() => media[selection] && (media[selection].ok = false)}
  />
</Portal>
