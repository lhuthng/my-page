<script>
	import MediaDirectory from './MediaDirectory.svelte';
	import MediaEditor from './MediaEditor.svelte';
	import MediaUploader from './MediaUploader.svelte';
	import { innerWidth } from 'svelte/reactivity/window';

	let { editMode, changeMode, ...rest } = $props();
	let keyword = $state('');
	let tab = $state(0);
	let detailPanel = $state();

	// Details panel slides in from the side on lg+, stacks below it.
	let isWide = $derived(innerWidth.current >= 1024);
</script>

<div {...rest}>
	<div class="flex flex-col py-4 gap-4 w-full">
		<div class="flex flex-wrap items-center justify-between gap-2">
			<div class="flex gap-2 flex-1 min-w-56 max-w-sm">
				<input
					disabled={editMode !== true}
					class="grow bg-background/40 border-2 border-dark/10 focus:border-dark rounded-xl px-3 py-1.5 text-base placeholder:text-dark/30 outline-none disabled:opacity-40"
					type="text"
					placeholder="Search keywords"
					bind:value={keyword}
				/>
				<button
					disabled={editMode !== true}
					class="rounded-xl border-2 border-dark/10 px-3 text-sm font-medium hover:bg-background/60 disabled:opacity-40 cursor-pointer"
					type="button"
				>
					Search
				</button>
			</div>

			<div class="flex gap-2">
				<button
					type="button"
					class="lg:hidden rounded-xl border-2 border-dark/10 px-3 py-1.5 text-sm font-medium hover:bg-background/60 cursor-pointer"
					onclick={() => (tab = tab === 1 ? 0 : 1)}
				>
					{tab === 1 ? 'Hide details' : 'Details'}
				</button>
				<div class="w-fit duo-btn" data-duo-color="primary">
					<button type="button" onclick={() => changeMode?.()}>
						{editMode ? 'Upload' : 'Edit'}
					</button>
				</div>
			</div>
		</div>

		<div class="relative z-9">
			<div
				class="relative flex flex-col lg:flex-row rounded-xl border-2 border-dark/10 overflow-hidden bg-white"
			>
				<div class="grow bg-background/20 min-h-96 lg:h-180 overflow-auto">
					{#if editMode}
						<MediaEditor {detailPanel} {keyword} openDetails={() => (tab = 1)} />
					{:else}
						<MediaUploader {detailPanel} openDetails={() => (tab = 1)} />
					{/if}
				</div>
				<div
					class="relative bg-white transition-all duration-200 border-dark/10 max-lg:border-t-2 lg:border-l-2"
					style:width={tab === 1 ? (isWide ? 'min(20rem, 50%)' : '100%') : '0'}
				>
					<div class="absolute w-80 max-w-full h-full p-1">
						<div bind:this={detailPanel}></div>
					</div>
				</div>
			</div>
			<!-- Vertical details tab handle (lg+); mobile uses the toolbar toggle -->
			<button
				type="button"
				class={`hidden lg:block absolute top-2 left-full z-9 rounded-r-lg border-2 border-l-0 border-dark/10 px-1 py-2 cursor-pointer transition-all duration-200 ${
					tab === 1
						? 'bg-dark text-white translate-x-0'
						: 'bg-background/60 text-dark -translate-x-1 hover:-translate-x-0.5 hover:bg-primary/40'
				}`}
				style:writing-mode="vertical-lr"
				onclick={() => (tab = tab === 1 ? 0 : 1)}
			>
				Details
			</button>
		</div>
	</div>
</div>
