<script>
	import { auth } from '$lib/auth/user.svelte.js';
	import { preventDefault } from '$lib/utils';
	import PBody from '../shell/PBody.svelte';

	let {
		show = false,
		apiPath = '',
		initialFile = null,
		onclose = () => {},
		onuploaded = (_data) => {}
	} = $props();

	const allowedTypes = [
		'image/jpeg',
		'image/png',
		'image/gif',
		'image/webp',
		'video/mp4',
		'video/webm'
	];
	const maxFileSize = 5 * 1024 * 1024;

	let coverFile = $state();
	let newCover = $state();
	let isUploading = $state(false);
	let isUploaded = $state(false);
	let coverError = $state('');
	let videoSeconds = $state(1);

	// `newCover` is a preview blob — but on a successful upload it is also the
	// exact value handed to the parent via `onuploaded` (this component never
	// gets a real URL back from the server, only the PATCH's success status),
	// and the parent stores it as the entry's live cover URL. So a blob only
	// gets revoked here while it's still just a local preview nobody else
	// references yet — never once `isUploaded` is true.
	function discardPreview() {
		if (newCover && !isUploaded) URL.revokeObjectURL(newCover);
	}

	$effect(() => {
		if (!show) {
			discardPreview();
			coverFile = undefined;
			newCover = undefined;
			isUploading = false;
			isUploaded = false;
			coverError = '';
			videoSeconds = 1;
		}
	});

	// Covers the case where the modal is torn down without `show` ever
	// flipping back to false (e.g. the whole editor unmounts while it's open).
	$effect(() => {
		return () => discardPreview();
	});

	// Shared by the dropzone and the `initialFile` preload (dropping straight
	// onto the sidebar cover card): validate, then show the preview blob.
	function acceptFile(file) {
		coverError = '';
		if (!allowedTypes.includes(file.type)) {
			coverError = 'Only JPEG, PNG, GIF, WEBP, MP4, or WebM are allowed.';
			return false;
		}
		if (file.size > maxFileSize) {
			coverError = `File size exceeds 5MB (${file.size} bytes)`;
			return false;
		}
		discardPreview();
		coverFile = file;
		newCover = URL.createObjectURL(file);
		isUploading = false;
		isUploaded = false;
		videoSeconds = 1;
		return true;
	}

	// A file dropped on the cover card is handed in via `initialFile` when the
	// modal opens, so the drop never has to survive a portal/overlay round-trip.
	$effect(() => {
		if (show && initialFile) acceptFile(initialFile);
	});

	const handleDrop = (e) => {
		e.preventDefault();
		const file = e.dataTransfer.files[0];
		if (!file) {
			coverError = 'File not found!';
			return;
		}
		acceptFile(file);
	};

	const upload = async () => {
		if (!coverFile || !apiPath) return;
		const formData = new FormData();
		formData.append('file', coverFile, coverFile.name);
		if (coverFile.type.startsWith('video/')) {
			formData.append('og_image_seconds', videoSeconds.toString());
		}
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
			onuploaded({
				url: newCover,
				ogImageSeconds: coverFile.type.startsWith('video/') ? videoSeconds : 0,
				fileType: coverFile.type
			});
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
						{#if coverFile?.type?.startsWith('video/')}
							<video class="full object-cover" src={newCover} muted controls></video>
						{:else}
							<img class="full object-cover" src={newCover} alt="cover-preview" />
						{/if}
					{:else}
						<span class="w-40 text-center select-none text-dark">
							Upload your image or video here
						</span>
					{/if}
				</div>
				{#if coverFile?.type?.startsWith('video/')}
					<div class="flex flex-col text-dark w-60">
						<label class="text-sm font-medium text-dark/60" for="video-seconds">
							Thumbnail second
						</label>
						<input
							id="video-seconds"
							type="number"
							class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white"
							bind:value={videoSeconds}
							min="0"
							step="0.1"
						/>
					</div>
				{/if}
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
