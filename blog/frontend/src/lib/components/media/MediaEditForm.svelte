<script>
	import { cache } from '$lib/utils/cache.svelte.js';
	import { auth } from '$lib/auth/user.svelte.js';

	import AliasList from './AliasList.svelte';
	import DeleteButton from './DeleteButton.svelte';
	import RevertButton from './RevertButton.svelte';

	let { shortName, onShortNameChanged } = $props();

	let details = $derived(cache.details[shortName]);
	let truth = $state();
	let draft = $state();

	$effect(() => {
		if (details) {
			const { short_name: shortName, description, file_type: fileType } = details.result;
			truth = { shortName, description, fileType };
			draft = { shortName, description, fileType };
		}
	});
</script>

<div class="flex flex-col gap-2 text-dark">
	{#if !details}
		<p class="text-sm text-dark/50 text-center py-4">
			Search and select any media to edit the file.
		</p>
	{:else if details?.status === 'waiting'}
		<p class="text-sm text-dark/50 text-center py-4 animate-pulse">Loading details…</p>
	{:else if draft && truth}
		<span class="text-center text-sm font-semibold text-dark/60 uppercase tracking-wide">Details</span>
		<form
			class="flex flex-col gap-2"
			method="patch"
			onsubmit={async () => {
				if (!truth) return;
				if (draft.shortName === truth.shortName && draft.description === truth.description) {
					console.log('no change detected.');
					return;
				}
				const body = {};
				if (draft.shortName !== truth.shortName) body.new_short_name = draft.shortName;
				if (draft.description !== truth.description) body.description = draft.description;
				const res = await fetch(`/api/media/d/${truth.shortName}`, {
					method: 'PATCH',
					headers: {
						'Content-Type': 'application/json',
						Authorization: auth()
					},
					body: JSON.stringify(body)
				});
				if (res.ok) {
					if (draft.shortName !== truth.shortName) {
						delete cache.details[truth.shortName];
						onShortNameChanged?.(draft.shortName);
					} else {
						cache.details[truth.shortName] = {
							...cache.details[truth.shortName],
							short_name: draft.shortName,
							description: draft.description
						};
					}
					truth = { ...draft };
				}
			}}
		>
			<fieldset class="border-2 border-dark/20 rounded-lg pt-1 pb-2 px-2">
				<legend class="font-semibold text-xs left-2 px-1" for="short-name">
					Short name{draft.shortName !== truth.shortName ? `*` : ''}
				</legend>
				<div class="flex w-full">
					<input
						class="grow rounded-lg border-2 border-dark/10 focus:border-dark px-2 py-1 bg-white"
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
			<fieldset class="border-2 border-dark/20 rounded-lg pt-1 pb-2 px-2">
				<legend class="font-semibold text-xs left-2 px-1" for="description">
					Description{draft.description !== truth.description ? `*` : ''}
				</legend>
				<div class="flex w-full">
					<textarea
						class="grow rounded-lg border-2 border-dark/10 focus:border-dark px-2 py-1 bg-white resize-none custom-scrollbar"
						type="text"
						rows="4"
						bind:value={draft.description}
						name="short-name"></textarea>
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
				<label class="text-sm text-dark/50" for="file-type">
					{draft.fileType}
				</label>
				<button
					class="ml-auto w-fit rounded-full border-2 border-dark/20 px-3 py-1 text-sm font-medium hover:bg-dark hover:text-white transition-colors cursor-pointer"
					type="submit"
				>
					Apply
				</button>
			</div>

			<DeleteButton type="button" class="ml-auto w-6 h-6 hover:scale-110" title="remove" />
		</form>
		<span class="text-center">Aliases</span>
		<AliasList {shortName} />
	{/if}
</div>
