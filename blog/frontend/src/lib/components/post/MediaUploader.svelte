<script>
	import { preventDefault } from '$lib/common';
	import MediumEntity from './MediumEntity.svelte';

	let { offlineMedia, onlineMedia, handlers, ...rest } = $props();

	const allowedTypes = [
		'image/png',
		'image/jpeg',
		'image/gif',
		'image/webp',
		'video/mp4',
		'video/webm',
		'audio/mpeg',
		'audio/ogg',
		'audio/wav',
		'model/gltf-binary',
		'application/vnd.lottie+zip',
		'application/zip',
		'application/x-zip-compressed',
		'application/x-zip',
		'application/octet-stream',
		'binary/octet-stream'
	];

	function isAllowedMedia(file) {
		if (!file) return false;
		if (allowedTypes.includes(file.type)) return true;
		return file.name.toLowerCase().endsWith('.lottie');
	}

	function addMedia(files) {
		const media = [];
		for (let index = 0; index < files.length; index++) {
			const file = files[index];
			if (isAllowedMedia(file)) {
				let { name, type } = file;
				let medium = {
					name,
					type,
					url: URL.createObjectURL(file),
					file
				};
				media.push(medium);
			}
		}
		handlers?.updateOfflineMedia(media);
	}

	function handleDrop(e) {
		e.preventDefault();
		addMedia(e.dataTransfer.files);
	}
</script>

<div {...rest}>
	<div class="full" ondrop={handleDrop} ondragover={preventDefault} role="listitem">
		{#if Object.keys(offlineMedia).length === 0}
			<div class="flex full rounded-lg border-dashed border-2 border-gray-400">
				<span class="block m-auto">Drop media here</span>
			</div>
		{:else}
			<ul class="full space-y-2">
				{#each Object.keys(offlineMedia)
					.sort()
					.map( (key) => ({ shortName: key, url: offlineMedia[key] }) ) as { shortName, url }, index (shortName)}
					<MediumEntity
						{shortName}
						{url}
						{handlers}
						warning={shortName in onlineMedia && onlineMedia[shortName]}
					/>
				{/each}
			</ul>
		{/if}
	</div>
</div>
