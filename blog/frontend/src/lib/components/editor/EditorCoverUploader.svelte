<script>
	import { auth } from '$lib/auth/user.svelte.js';
	import { preventDefault } from '$lib/utils';
	import PBody from '../shell/PBody.svelte';

	let { show = false, apiPath = '', onclose = () => {}, onuploaded = (_url) => {} } = $props();

	const allowedTypes = ['image/jpeg', 'image/png', 'image/gif', 'image/webp'];
	const maxFileSize = 5 * 1024 * 1024;

	let coverFile = $state();
	let newCover = $state();
	let isUploading = $state(false);
	let isUploaded = $state(false);
	let coverError = $state('');

	$effect(() => {
		if (!show) {
			coverFile = undefined;
			newCover = undefined;
			isUploading = false;
			isUploaded = false;
			coverError = '';
		}
	});

	const handleDrop = (e) => {
		e.preventDefault();
		const file = e.dataTransfer.files[0];
		if (!file) {
			coverError = 'File not found!';
			return;
		}
		if (!allowedTypes.includes(file.type)) {
			coverError = 'Only JPEG, PNG, GIF, or WEBP are allowed.';
			return;
		}
		if (file.size > maxFileSize) {
			coverError = `File size exceeds 5MB (${file.size} bytes)`;
			return;
		}
		coverFile = file;
		newCover = URL.createObjectURL(file);
		isUploading = false;
		isUploaded = false;
	};

	const upload = async () => {
		if (!coverFile || !apiPath) return;
		const formData = new FormData();
		formData.append('file', coverFile, coverFile.name);
		isUploaded = false;
		isUploading = true;
		const res = await fetch(apiPath, {
			method: 'PATCH',
			headers: { Authorization: auth() },
			body: formData
		});
		if (res.ok) {
			isUploaded = true;
			isUploading = false;
			coverError = '';
			onuploaded(newCover);
		} else {
			isUploading = false;
			coverError = await res.text();
		}
	};
</script>

{#if show}
	<PBody>
		<div class="sticky top-0 flex justify-center items-center w-full h-screen pointer-events-auto">
			<div class="absolute cursor-not-allowed inset-0 z-10" onclick={onclose} role="none"></div>
			<div class="w-fit h-fit space-y-4 bg-white rounded-3xl p-4 text-xl z-11" role="none">
				<div
					class="flex justify-center items-center w-60 h-60 bg-background/60 outline-4 outline-dark outline-dashed rounded-xl overflow-hidden"
					ondrop={handleDrop}
					ondragover={preventDefault}
					role="none"
				>
					{#if newCover}
						<img class="full object-cover" src={newCover} alt="cover-preview" />
					{:else}
						<span class="w-40 text-center select-none text-dark">Upload your image here</span>
					{/if}
				</div>
				{#if coverError}
					<span class="inline-block w-60 text-accent-red">*{coverError}</span>
				{/if}
				<div class="duo-btn" data-duo-color="green">
					<button disabled={!newCover || isUploaded || isUploading} onclick={upload}>Apply</button>
				</div>
				{#if isUploaded}
					<span class="inline-block w-60 text-accent-green">*New cover uploaded successfully!</span>
				{/if}
			</div>
		</div>
	</PBody>
{/if}
