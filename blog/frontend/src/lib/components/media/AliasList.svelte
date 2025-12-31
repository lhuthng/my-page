<script>
  import { detailsCache } from "$lib/client/details-cache";
  import { auth } from "$lib/client/user";
  import DeleteButton from "./DeleteButton.svelte";
  import RevertButton from "./RevertButton.svelte";

  let { shortName } = $props();

  let details = $derived($detailsCache[shortName]);
  let draft = $state([]);
  let pending = $state();

  $effect(() => {
    if (details && shortName !== pending) {
      const { aliases } = details.result;
      draft = (aliases ?? []).map((alias) => ({
        name: alias,
        origin: alias,
      }));
      pending = shortName;
    }
  });

  async function onsubmit() {
    draft.forEach(async (alias) => {
      switch (alias.action) {
        case "created": {
          const res = await fetch(`/api/media/d/${shortName}/aliases`, {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
              Authorization: auth(),
            },
            body: JSON.stringify({
              alias: alias.name,
            }),
          });
          if (res.ok) {
            alias.res = { status: "success" };
            alias.origin = alias.name;
            alias.action = undefined;

            const aliases = details?.result?.aliases;
            if (aliases) {
              aliases.push(alias.name);
            }
          } else {
            alias.res = {
              status: "failed",
              text: await res.text(),
            };
          }
          break;
        }
        case "modified": {
          const res = await fetch(
            `/api/media/d/${shortName}/aliases/${alias.origin}`,
            {
              method: "PATCH",
              headers: {
                "Content-Type": "application/json",
                Authorization: auth(),
              },
              body: JSON.stringify({
                new_alias: alias.name,
              }),
            },
          );
          if (res.ok) {
            alias.res = { status: "success" };
            alias.origin = alias.name;
            alias.action = undefined;

            const aliases = details?.result?.aliases;
            if (aliases) {
              const index = aliases.indexOf(alias.origin);
              if (index !== -1) {
                aliases[index] = alias.name;
              }
            }
          }
          break;
        }
        case "deleted": {
          const res = await fetch(
            `/api/media/d/${shortName}/aliases/${alias.origin}`,
            {
              method: "DELETE",
              headers: {
                Authorization: auth(),
              },
            },
          );
          if (res.ok) {
            const aliases = details?.result?.aliases;
            if (aliases) {
              const index = aliases.indexOf(alias.origin);
              if (index !== -1) {
                aliases.splice(index, 1);
              }
            }
          }
          break;
        }
        default:
          console.log("Unhandled action", alias.action);
      }
    });
  }
</script>

{#snippet remove(cls, onclick, undo)}
  <button type="button" class={cls} {onclick} title="remove">
    <svg
      xmlns="http://www.w3.org/2000/svg"
      x="0px"
      y="0px"
      viewBox="0 0 24 24"
      fill="none"
    >
      <path
        d="M7 4a2 2 0 0 1 2-2h6a2 2 0 0 1 2 2v2h4a1 1 0 1 1 0 2h-1.069l-.867 12.142A2 2 0 0 1 17.069 22H6.93a2 2 0 0 1-1.995-1.858L4.07 8H3a1 1 0 0 1 0-2h4V4zm2 2h6V4H9v2zM6.074 8l.857 12H17.07l.857-12H6.074zM10 10a1 1 0 0 1 1 1v6a1 1 0 1 1-2 0v-6a1 1 0 0 1 1-1zm4 0a1 1 0 0 1 1 1v6a1 1 0 1 1-2 0v-6a1 1 0 0 1 1-1z"
        fill="#0D0D0D"
      />
      {#if undo}
        <path
          d="M4 4 L20 20"
          stroke="#0D0D0D"
          stroke-width="2"
          stroke-linecap="round"
        />
      {/if}
    </svg>
  </button>
{/snippet}

<form class="flex flex-col space-y-3" type="submit" {onsubmit}>
  <ul class="space-y-2">
    {#each draft as alias, index}
      <li>
        <fieldset
          class="border-2 rounded-lg pb-1 px-2 focus-within:border-blue-500 focus-within:text-blue-500"
        >
          <legend class="bg-white font-semibold text-xs">
            {#if alias.origin !== undefined && alias.origin !== alias.name}
              <span class="px-1">{`<${alias.origin}>`}</span>
            {:else if alias.origin === undefined}
              <span class="px-1">*new</span>
            {:else}
              <span>&ZeroWidthSpace;</span>
            {/if}
          </legend>
          <div class="w-full flex">
            <input
              disabled={alias.action === "deleted"}
              class="focus:outline-0 grow disabled:text-gray-400 disabled:line-through"
              type="text"
              bind:value={
                () => alias.name,
                (v) => {
                  alias.name = v;
                  if (alias.origin) {
                    alias.action = "modified";
                    if (alias.origin === alias.name) alias.action = undefined;
                  }
                }
              }
            />
            <div class="flex items-center gap-2 pl-2">
              <RevertButton
                type="button"
                class="w-6 h-6 not-disabled:hover:scale-110 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
                onclick={() => {
                  alias.name = alias.origin;
                  if (alias.origin) {
                    alias.action = undefined;
                  } else {
                    alias.action = "created";
                  }
                }}
                title="revert"
                disabled={alias.action === "deleted" ||
                  alias.origin === undefined ||
                  (alias.origin !== undefined && alias.origin === alias.name)}
              />
              <DeleteButton
                type="button"
                class="w-6 h-6 not-disabled:hover:scale-110 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
                title="remove"
                disabled={alias.action === "deleted"}
                onclick={(e) => {
                  e.preventDefault();
                  if (alias.origin) {
                    if (alias.action !== "deleted") {
                      alias.action = "deleted";
                    } else {
                      alias.action =
                        alias.origin === alias.name ? undefined : "modified";
                    }
                  } else {
                    draft.splice(index, 1);
                  }
                }}
              />
            </div>
          </div>
        </fieldset>
      </li>
    {/each}
  </ul>
  <button
    class="mx-auto w-fit ring-2 rounded-xl px-2 text-sm hover:cursor-pointer"
    type="button"
    onclick={() =>
      draft.push({
        name: "new-alias",
        action: "created",
      })}
  >
    +
  </button>
  <button
    class="ml-auto w-fit border-2 px-1 rounded-lg cursor-pointer"
    type="submit"
  >
    Apply
  </button>
</form>
