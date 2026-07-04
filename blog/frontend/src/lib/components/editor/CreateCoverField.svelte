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

<div class="flex flex-col gap-2">
	<label for="create-cover">Cover media:</label>
	<input
		id="create-cover"
		type="file"
		accept="image/jpeg,image/png,image/gif,image/webp,video/mp4,video/webm"
		onchange={(e) => onselect(e.currentTarget.files?.[0])}
		disabled={!isOwner}
	/>
	{#if file?.type?.startsWith('video/')}
		<div class="flex flex-col">
			<label for="create-cover-seconds">Thumbnail second:</label>
			<input
				id="create-cover-seconds"
				type="number"
				class="px-1 min-w-0 bg-white rounded-sm"
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
