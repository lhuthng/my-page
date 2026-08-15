<script>
	let {
		file = undefined,
		error = '',
		ogImageSeconds = 0,
		isOwner = true,
		onselect = (_file) => {},
		onsecondschange = (_seconds) => {}
	} = $props();
</script>

<div class="flex flex-col mt-2 gap-2">
	<label class="text-sm font-medium text-dark/60" for="create-cover">Cover media</label>
	<input
		id="create-cover"
		type="file"
		accept="image/jpeg,image/png,image/gif,image/webp,video/mp4,video/webm"
		class="block w-full rounded-xl border-2 border-dark px-3 py-2 text-sm text-dark outline-none transition-colors focus:bg-primary focus:text-white file:mr-3 file:cursor-pointer file:rounded-lg file:border-0 file:bg-primary file:px-3 file:py-1.5 file:font-medium file:text-white disabled:opacity-60"
		onchange={(e) => onselect(e.currentTarget.files?.[0])}
		disabled={!isOwner}
	/>
	{#if file?.type?.startsWith('video/')}
		<div class="flex flex-col gap-1">
			<label class="text-sm font-medium text-dark/60" for="create-cover-seconds">
				Thumbnail second
			</label>
			<input
				id="create-cover-seconds"
				type="number"
				class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white disabled:opacity-60"
				value={ogImageSeconds}
				oninput={(e) => onsecondschange(Number(e.currentTarget.value))}
				min="0"
				step="0.1"
				readonly={!isOwner}
			/>
		</div>
	{/if}
	{#if error}
		<p class="text-sm text-accent-red">{error}</p>
	{/if}
</div>
