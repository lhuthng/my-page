<script>
	import { onDestroy } from 'svelte';
	import { preventDefault } from '$lib/utils';
	import { createEditorViewModel } from '$lib/features/editor/view-model/create-editor-vm.svelte.js';
	import { DEMO_TYPES } from '$lib/features/editor/model/demo.js';
	import PostEditorShell from '../editor/PostEditorShell.svelte';

	let { mode = 'create', data, isOwner = true, games = [] } = $props();

	const vm = createEditorViewModel({ mode, kind: 'project', data, isOwner });
	onDestroy(() => vm.destroy());

	const demoType = $derived(vm.entry.demoType);
	const urlLabel = $derived(
		demoType === 'embed'
			? 'Demo URL (required)'
			: demoType === 'download'
				? 'Download URL (required)'
				: 'Video URL (required)'
	);
	const urlPlaceholder = $derived(
		demoType === 'video'
			? 'https://example.com/video.mp4'
			: demoType === 'download'
				? 'https://example.com/download-link'
				: 'https://example.github.io/my-demo/'
	);
</script>

{#snippet extraFields()}
	<div class="flex flex-col gap-1">
		<label class="text-sm font-medium text-dark/60" for="demo-type">Demo type</label>
		<select
			id="demo-type"
			class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white disabled:opacity-60"
			value={vm.entry.demoType}
			onchange={(e) => vm.setDemoType(e.currentTarget.value)}
			disabled={!isOwner}
		>
			{#each DEMO_TYPES as type}
				<option value={type.value} disabled={type.disabled}>
					{type.label}{type.disabled ? ' (soon)' : ''}
				</option>
			{/each}
		</select>
	</div>
	{#if demoType === 'game'}
		<div class="flex flex-col gap-3 rounded-xl border border-background bg-background/20 p-3">
			<div class="flex flex-col gap-1">
				<label class="text-sm font-medium text-dark/60" for="delegate-game">Delegate to game</label>
				<select
					id="delegate-game"
					class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white disabled:opacity-60"
					bind:value={vm.entry.delegateGameId}
					disabled={!isOwner}
				>
					<option value="">Select a game</option>
					{#each games as game}
						<option value={game.id}>{game.title}</option>
					{/each}
				</select>
			</div>
			<p class="text-sm leading-relaxed text-dark/50">
				The project plays the selected game's launcher. Its playable demo lives on the game — edit
				it from the game's own dashboard page.
			</p>
			<label class="flex items-center gap-2 text-sm text-dark/70">
				<input type="checkbox" bind:checked={vm.entry.inheritThumbnail} disabled={!isOwner} />
				Inherit the game's thumbnail
			</label>
			<label class="flex items-center gap-2 text-sm text-dark/70">
				<input type="checkbox" bind:checked={vm.entry.inheritTags} disabled={!isOwner} />
				Inherit the game's tags
			</label>
		</div>
	{/if}
	{#if demoType !== 'none' && demoType !== 'game'}
		<div class="grid grid-cols-2 gap-2">
			<div class="flex flex-col gap-1">
				<label class="text-sm font-medium text-dark/60" for="demo-width">Demo width</label>
				<input
					id="demo-width"
					class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white"
					bind:value={vm.entry.demoWidth}
					readonly={!isOwner}
				/>
			</div>
			<div class="flex flex-col gap-1">
				<label class="text-sm font-medium text-dark/60" for="demo-height">Demo height</label>
				<input
					id="demo-height"
					class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white"
					bind:value={vm.entry.demoHeight}
					readonly={!isOwner}
				/>
			</div>
		</div>
	{/if}
	{#if demoType === 'embed' || demoType === 'download' || demoType === 'video'}
		<div class="flex flex-col gap-1">
			<label class="text-sm font-medium text-dark/60" for="demo-url">{urlLabel}</label>
			<input
				id="demo-url"
				class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white placeholder:text-dark/30 focus:placeholder:text-white/70"
				bind:value={vm.entry.demoUrl}
				placeholder={urlPlaceholder}
				readonly={!isOwner}
			/>
		</div>
	{/if}
	{#if demoType === 'html5' || demoType === 'webgl'}
		<div
			class="rounded-xl border-2 border-dashed border-dark/30 bg-background/20 p-3"
			ondrop={(e) => {
				e.preventDefault();
				vm.setDemoZip(e.dataTransfer.files[0]);
			}}
			ondragover={preventDefault}
			role="none"
		>
			<label class="block text-sm font-medium text-dark/60" for="demo-zip">
				{demoType === 'html5' ? 'HTML5 zip' : 'WebGL zip'}
			</label>
			<input
				id="demo-zip"
				type="file"
				accept=".zip,application/zip"
				disabled={!isOwner}
				class="mt-1 block w-full rounded-xl border-2 border-dark px-3 py-2 text-sm text-dark outline-none transition-colors focus:bg-primary focus:text-white file:mr-3 file:cursor-pointer file:rounded-lg file:border-0 file:bg-primary file:px-3 file:py-1.5 file:font-medium file:text-white"
				onchange={(e) => vm.setDemoZip(e.currentTarget.files?.[0])}
			/>
			{#if vm.ui.demoZipName}
				<p class="mt-1 text-sm text-accent-green">{vm.ui.demoZipName}</p>
			{:else if mode === 'edit'}
				<p class="mt-1 text-sm text-dark/50">Leave empty to keep current demo.</p>
			{/if}
			{#if vm.ui.demoZipError}
				<p class="mt-1 text-sm text-accent-red">{vm.ui.demoZipError}</p>
			{/if}
		</div>
	{/if}
	<div class="flex flex-col gap-2">
		<div class="flex items-center justify-between">
			<span class="text-sm font-medium text-dark/70">External links</span>
			<button
				class="flex h-7 w-7 items-center justify-center rounded-lg border border-background bg-background/40 text-lg leading-none text-dark transition-colors hover:bg-background/60 disabled:opacity-50"
				disabled={!isOwner}
				onclick={() => (vm.entry.links = [...vm.entry.links, { label: '', url: '' }])}
			>
				+
			</button>
		</div>
		{#each vm.entry.links as link, index}
			<div class="grid grid-cols-[1fr_1fr_auto] gap-1.5 items-center">
				<input
					class="w-full min-w-0 rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white placeholder:text-dark/30 focus:placeholder:text-white/70"
					placeholder="Label"
					bind:value={link.label}
					readonly={!isOwner}
				/>
				<input
					class="w-full min-w-0 rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white placeholder:text-dark/30 focus:placeholder:text-white/70"
					placeholder="URL"
					bind:value={link.url}
					readonly={!isOwner}
				/>
				<button
					class="flex h-7 w-7 items-center justify-center rounded-lg border border-accent-red/30 bg-accent-red-light-4 text-accent-red transition-colors hover:bg-accent-red-light-3 disabled:opacity-50"
					disabled={!isOwner}
					onclick={() => (vm.entry.links = vm.entry.links.filter((_, i) => i !== index))}
				>
					✕
				</button>
			</div>
		{/each}
	</div>
{/snippet}

<PostEditorShell
	{vm}
	kind="project"
	coverApiPath={`/api/projects/id/${vm.entry.id}/cover`}
	titleLabel="Name"
	excerptRows={4}
	{extraFields}
/>
