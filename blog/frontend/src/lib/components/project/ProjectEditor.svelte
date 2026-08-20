<script>
	import { onDestroy } from 'svelte';
	import { preventDefault } from '$lib/utils';
	import { createEditorViewModel } from '$lib/features/editor/view-model/create-editor-vm.svelte.js';
	import { DEMO_TYPES } from '$lib/features/editor/model/demo.js';
	import PostEditorShell from '../editor/PostEditorShell.svelte';

	let { mode = 'create', data, isOwner = true, v86Systems = [] } = $props();

	const vm = createEditorViewModel({ mode, kind: 'project', data, isOwner });
	onDestroy(() => vm.destroy());
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
	{#if vm.entry.demoType !== 'none'}
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
	{#if vm.entry.demoType !== 'none' && vm.entry.demoType !== 'html5' && vm.entry.demoType !== 'webgl' && vm.entry.demoType !== 'jsdos' && vm.entry.demoType !== 'v86'}
		<div class="flex flex-col gap-1">
			<label class="text-sm font-medium text-dark/60" for="demo-url">
				{#if vm.entry.demoType === 'embed'}
					Demo URL (required)
				{:else if vm.entry.demoType === 'download'}
					Download URL (required)
				{:else if vm.entry.demoType === 'video'}
					Video URL (required)
				{:else}
					URL
				{/if}
			</label>
			<input
				id="demo-url"
				class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white placeholder:text-dark/30 focus:placeholder:text-white/70"
				bind:value={vm.entry.demoUrl}
				placeholder={vm.entry.demoType === 'video'
					? 'https://example.com/video.mp4'
					: vm.entry.demoType === 'download'
						? 'https://example.com/download-link'
						: 'https://example.github.io/my-demo/'}
				readonly={!isOwner}
			/>
		</div>
	{/if}
	{#if vm.entry.demoType === 'v86'}
		<div class="flex flex-col gap-3 rounded-xl border border-background bg-background/20 p-3">
			<div class="flex flex-col gap-1">
				<label class="text-sm font-medium text-dark/60" for="v86-system">System</label>
				<select
					id="v86-system"
					class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white disabled:opacity-60"
					bind:value={vm.entry.v86SystemVersionId}
					disabled={!isOwner}
				>
					<option value="">Select a system</option>
					{#each v86Systems as system}
						{#each system.versions as version}
							<option value={version.id}>
								{system.name} v{version.version_number}{system.is_default ? ' (default)' : ''}
							</option>
						{/each}
					{/each}
				</select>
			</div>
			<div class="flex flex-col gap-4">
				<label class="text-sm font-medium text-dark/60" for="v86-manifest">Manifest</label>
				<textarea
					id="v86-manifest"
					rows="6"
					class="w-full rounded-xl px-3 py-2 font-mono text-sm text-dark outline-none border-2 border-dark transition-colors resize-none custom-scrollbar focus:bg-primary focus:text-white"
					bind:value={vm.entry.v86Manifest}
					readonly={!isOwner}></textarea>
				<p class="text-sm leading-relaxed text-dark/50">
					v86 manifest keys: exe (required), plus optional args, delay_ms, save_paths,
					revert_mouse_y (1 inverts the mouse's Y axis) and mouse_speed (a speed multiplier, e.g.
					2.0). Paths are relative to the game drive root.
				</p>
				<p class="text-sm leading-relaxed text-dark/50">
					Variants: name / name1, name2, name3… define launch variants (names must be contiguous).
					Each variant's exe2/args2 falls back to the root exe/args when omitted.
				</p>
			</div>
			{#if mode === 'edit' && data.id}
				<div class="flex flex-col gap-1 border-t border-dark/10 pt-3">
					<span class="text-sm font-medium text-dark/70">Boot snapshot</span>
					<p class="text-sm leading-relaxed text-dark/50">
						Capture an already-booted machine so visitors skip the Windows boot sequence. Only
						available once the game disk is attached, and it must be recaptured whenever the base or
						game disk changes.
					</p>
					<a
						class="text-sm font-medium text-accent-blue-dark hover:underline"
						href="/dashboard/projects/id/{data.id}/snapshot"
					>
						Open the snapshot studio →
					</a>
				</div>
			{/if}
		</div>
	{/if}
	{#if vm.entry.demoType === 'html5' || vm.entry.demoType === 'webgl' || vm.entry.demoType === 'jsdos' || vm.entry.demoType === 'v86'}
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
				{vm.entry.demoType === 'jsdos'
					? 'js-dos bundle'
					: vm.entry.demoType === 'v86'
						? 'v86 game ZIP'
						: vm.entry.demoType === 'html5'
							? 'HTML5 zip'
							: 'WebGL zip'}
			</label>
			<input
				id="demo-zip"
				type="file"
				accept={vm.entry.demoType === 'jsdos' ? '.jsdos' : '.zip,application/zip'}
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
