<script>
	import { auth } from '$lib/auth/user.svelte.js';

	let { media, onsuccess, onfailed } = $props();

	let draft = $state({});

	$effect(() => {
		if (media !== undefined) {
			const { file, name, description, type } = media;
			draft = {
				shortName: '',
				file,
				name,
				description,
				type
			};
		}
	});
</script>

<div class="flex flex-col gap-2 text-dark">
	{#if !media}
		<p class="text-sm text-dark/50 text-center py-4">Select an uploaded media to edit it.</p>
	{:else}
		<span class="text-center text-sm font-semibold text-dark/60 uppercase tracking-wide">Details</span>
		<form
			class="flex flex-col gap-2"
			method="post"
			onsubmit={async (e) => {
				e.preventDefault();

				if (!media) return;

				const formData = new FormData();

				formData.append('file', draft.file, draft.name);
				formData.append('short_name', draft.shortName);
				formData.append('description', draft.description);

				const res = await fetch('/api/media/upload', {
					method: 'POST',
					headers: {
						Authorization: auth()
					},
					body: formData
				});

				if (res.ok) {
					onsuccess?.();
				} else {
					onfailed?.();
				}
			}}
		>
			<label class="ml-auto mr-4 text-sm text-dark/50" for="file-type">
				{draft.type}
			</label>
			<fieldset class="border-2 border-dark/20 rounded-lg pt-1 pb-2 px-2">
				<legend class="font-semibold text-xs left-2 px-1" for="short-name">Short name</legend>
				<input
					disabled={media?.ok}
					class="w-full rounded-lg border-2 border-dark/10 focus:border-dark px-2 py-1 bg-white"
					type="text"
					bind:value={draft.shortName}
					name="short-name"
				/>
			</fieldset>
			<fieldset class="border-2 border-dark/20 rounded-lg pt-1 pb-2 px-2">
				<legend class="font-semibold text-xs left-2 px-1" for="description">Description</legend>
				<textarea
					disabled={media?.ok}
					class="w-full rounded-lg border-2 border-dark/10 focus:border-dark px-2 py-1 bg-white resize-none custom-scrollbar"
					type="text"
					rows="4"
					bind:value={draft.description}
					name="short-name"></textarea>
			</fieldset>
			<button
				disabled={media?.ok}
				class="ml-auto w-fit rounded-full border-2 border-dark/20 px-3 py-1 text-sm font-medium hover:bg-dark hover:text-white transition-colors disabled:opacity-40 cursor-pointer"
				type="submit"
			>
				Submit
			</button>
		</form>
	{/if}
</div>
