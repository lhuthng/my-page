<script>
	import ImageEntity from "./ImageEntity.svelte";

	const maxLength = 32;

	let { mediaDictionary = $bindable() } = $props();

	let imageList = $state([]);
	let renamingIndex = $state(undefined);
	let errorLogs = $state([]);

	function appendImages(files) {
		let images = [];
		const newDict = { ...mediaDictionary };
		for (let index = 0; index < files.length; index++) {
			const file = files[index];
			if (file && file.type.startsWith("image/")) {
				let name = file.name
					.substr(0, file.name.lastIndexOf("."))
					.trim()
					.slice(0, maxLength);
				let oversize = false;
				while (name in newDict) {
					if (name.length < maxLength) {
						name = name + "*";
					} else {
						oversize = true;
						break;
					}
				}
				if (oversize) {
					errorLogs.push(
						`Cannot add ${file.name}, length must be less than ${maxLength}.`,
					);
					continue;
				}
				let image = {
					name: name,
					type: file.type.substr(6),
					url: URL.createObjectURL(file),
				};
				images.push(image);
				newDict[image.name] = image.url;
			}
		}
		mediaDictionary = { ...newDict };
		imageList = imageList.concat(images.map((image) => image.name));
	}

	function handleDrop(e) {
		e.preventDefault();
		appendImages(e.dataTransfer.files);
	}

	function handleFile(e) {
		appendImages(e.dataTransfer.files);
	}

	function handleDragOver(e) {
		e.preventDefault();
	}

	const entityHandlers = {
		remove(index) {
			const newDict = { ...mediaDictionary };
			const name = imageList[index];
			const url = newDict[name];
			URL.revokeObjectURL(url);
			imageList.splice(index, 1);
			delete newDict[name];

			mediaDictionary = { ...newDict };
		},
		upward(index) {
			if (index <= 0) return;
			[imageList[index], imageList[index - 1]] = [
				imageList[index - 1],
				imageList[index],
			];
		},
		downward(index) {
			if (index >= imageList.length - 1) return;
			[imageList[index], imageList[index + 1]] = [
				imageList[index + 1],
				imageList[index],
			];
		},
		checkName(index, name) {
			return (
				!(name.trim() in mediaDictionary) ||
				imageList[index] === name.trim()
			);
		},
		startRenaming(index) {
			renamingIndex = index;
			return true;
		},
		stopRenaming(index) {
			renamingIndex = undefined;
		},
		submitRename(index, newName) {
			if (this.checkName(index, newName)) {
				const oldName = imageList[index];
				const image = mediaDictionary[oldName];
				const newDict = { ...mediaDictionary };
				delete newDict[oldName];
				imageList[index] = newName;
				newDict[newName] = image;
				if (renamingIndex === index) {
					renamingIndex = undefined;
				}

				mediaDictionary = { ...newDict };
			}
		},
	};
</script>

<p>Your image(s):</p>
<div
	class="w-full border-2 border-dashed border-gray-400 grid grid-cols-[repeat(auto-fit,minmax(20rem,1fr))] gap-2 items-center text-gray-500 bg-gray-200 rounded-lg p-1"
	ondrop={handleDrop}
	ondragover={handleDragOver}
	role={"list"}
>
	{#if imageList.length === 0}
		<div class="m-auto text-center">
			<p>Drop an image (PNG, WEBP, JPG...) here</p>
			<p class="text-sm text-gray-400 my-1">or</p>
			<!-- <input
                class="text-center"
                type="file"
                accept="image/*"
                onchange={handleFile}
            /> -->
		</div>
	{:else}
		{#each imageList as name, index (name)}
			<ImageEntity
				{index}
				{name}
				isRenaming={index === renamingIndex}
				length={imageList.length}
				dictionary={mediaDictionary}
				handlers={entityHandlers}
			/>
		{/each}
	{/if}
</div>
