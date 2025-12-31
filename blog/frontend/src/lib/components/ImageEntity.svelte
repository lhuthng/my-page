<script>
  let { index, length, name, isRenaming, dictionary, handlers } = $props();

  let newName = $state(name);
  let isValid = $derived(handlers?.checkName(index, newName));

  function handleKeyDown(e) {
    const key = e.key;
    if (key === "Enter") {
      event.preventDefault();
      handlers?.submitRename(index, newName);
    } else if (key === "Escape") {
      handlers?.stopRenaming(index);
      newName = name;
    }
  }
</script>

<div class="flex w-full h-full gap-2 bg-gray-100 rounded-sm p-1">
  <span class="my-auto">{index + 1}.</span>
  <img
    class="w-20 h-20 rounded-sm ring-1 object-contain"
    src={dictionary[name]}
    alt={name}
  />
  <div class="flex flex-col-reverse justify-between grow p-1">
    <div class="flex gap-4">
      <button
        class="text-blue-500 cursor-pointer hover:brightness-140"
        style:text-decoration={isRenaming ? "underline" : "none"}
        onclick={() => handlers?.startRenaming(index)}>edit</button
      >
      <button
        class="text-blue-500 cursor-pointer hover:brightness-140"
        onclick={() => handlers?.remove(index)}>delete</button
      >
      {#if isRenaming}
        <button
          class="text-blue-500 cursor-pointer hover:brightness-140"
          onclick={() => handlers?.submitRename(index, newName)}>ok</button
        >
      {/if}
    </div>
    <div class="flex flex-col grow justify-center font-mono">
      {#if isRenaming}
        <textarea
          class={`w-full resize-none rounded-sm overflow-hidden ${isValid ? "bg-green-200" : "bg-red-100"}`}
          bind:value={newName}
          rows="2"
          onkeydown={handleKeyDown}
        ></textarea>
      {:else}
        <span style:word-break="break-all">{name}</span>
      {/if}
    </div>
  </div>
  <div
    class="flex flex-col justify-between w-4 h-full [&>svg]:transition-transform [&>svg]:duration-100 [&>svg]:origin-center"
  >
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
      class={index > 0 ? "cursor-pointer hover:scale-110" : "opacity-30"}
      onclick={() => handlers?.upward(index)}
    >
      <polyline
        points="4,10 8,6 12,10"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        fill="none"
      />
    </svg>
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
      class={index < length - 1
        ? "cursor-pointer hover:scale-110"
        : "opacity-30"}
      onclick={() => handlers?.downward(index)}
    >
      <polyline
        points="4,6 8,10 12,6"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        fill="none"
      />
    </svg>
  </div>
</div>
