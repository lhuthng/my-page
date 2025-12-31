<script>
  import { detailsCache } from "$lib/client/details-cache";
  import { auth } from "$lib/client/user";

  import AliasList from "./AliasList.svelte";
  import DeleteButton from "./DeleteButton.svelte";
  import RevertButton from "./RevertButton.svelte";

  let { shortName, onShortNameChanged } = $props();

  let details = $derived($detailsCache[shortName]);
  let truth = $state();
  let draft = $state();

  $effect(() => {
    if (details) {
      const {
        short_name: shortName,
        description,
        file_type: fileType,
      } = details.result;
      truth = { shortName, description, fileType };
      draft = { shortName, description, fileType };
    }
  });
</script>

<div class="flex flex-col gap-2">
  {#if !details}
    <span>Search and select any media to edit the file</span>
  {:else if details?.status === "waiting"}
    <span>Waiting, I'm loading</span>
  {:else if draft && truth}
    <span class="text-center">Details</span>
    <form
      class="flex flex-col gap-2"
      method="patch"
      onsubmit={async () => {
        if (!truth) return;
        if (
          draft.shortName === truth.shortName &&
          draft.description === truth.description
        ) {
          console.log("no change detected.");
          return;
        }
        const body = {};
        if (draft.shortName !== truth.shortName)
          body.new_short_name = draft.shortName;
        if (draft.description !== truth.description)
          body.description = draft.description;
        const res = await fetch(`/api/media/d/${truth.shortName}`, {
          method: "PATCH",
          headers: {
            "Content-Type": "application/json",
            Authorization: auth(),
          },
          body: JSON.stringify(body),
        });
        if (res.ok) {
          if (draft.shortName !== truth.shortName) {
            delete detailsCache[truth.shortName];
            onShortNameChanged?.(draft.shortName);
          } else {
            detailsCache[truth.shortName] = {
              ...detailsCache[truth.shortName],
              short_name: draft.shortName,
              description: draft.description,
            };
          }
          truth = { ...draft };
        }
      }}
    >
      <fieldset class="border-2 rounded-lg pt-1 pb-2 px-2">
        <legend class="font-semibold text-xs left-2 px-1" for="short-name">
          Short name{draft.shortName !== truth.shortName ? `*` : ""}
        </legend>
        <div class="flex w-full">
          <input
            class="grow focus:outline-0"
            type="text"
            bind:value={draft.shortName}
            name="short-name"
          />
          <RevertButton
            class="w-6 h-6 not-disabled:hover:scale-110 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
            title="revert"
            type="button"
            onclick={() => (draft.shortName = truth.shortName)}
            disabled={draft.shortName === truth.shortName}
          />
        </div>
      </fieldset>
      <fieldset class="border-2 rounded-lg pt-1 pb-2 px-2">
        <legend class="font-semibold text-xs left-2 px-1" for="description">
          Description{draft.description !== truth.description ? `*` : ""}
        </legend>
        <div class="flex w-full">
          <textarea
            class="grow focus:outline-0 resize-none"
            type="text"
            rows="4"
            bind:value={draft.description}
            name="short-name"
          ></textarea>
          <RevertButton
            class="w-6 h-6 not-disabled:hover:scale-110 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
            title="revert"
            type="button"
            onclick={() => (draft.description = truth.description)}
            disabled={draft.description === truth.description}
          />
        </div>
      </fieldset>
      <div class="flex w-full">
        <label class="text-sm" for="file-type">
          {draft.fileType}
        </label>
        <button
          class="ml-auto w-fit border-2 px-1 rounded-lg cursor-pointer"
          type="submit"
        >
          Apply
        </button>
      </div>

      <DeleteButton
        type="button"
        class="ml-auto w-6 h-6 hover:scale-110"
        title="remove"
      />
    </form>
    <span class="text-center">Aliases</span>
    <AliasList {shortName} />
  {/if}
</div>
