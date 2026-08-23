<script>
	import { onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { preventDefault } from '$lib/utils';
	import { createEditorViewModel } from '$lib/features/editor/view-model/create-editor-vm.svelte.js';
	import { GAME_DEMO_TYPES } from '$lib/features/editor/model/demo.js';
	import PostEditorShell from '../editor/PostEditorShell.svelte';
	import ConfirmDialog from '../ui/ConfirmDialog.svelte';
	import { api } from '$lib/api/client.js';

	let { mode = 'create', data, isOwner = true, v86Systems = [], games = [] } = $props();

	const vm = createEditorViewModel({ mode, kind: 'game', data, isOwner });
	onDestroy(() => vm.destroy());

	let showDelete = $state(false);
	let deleteBusy = $state(false);
	let deleteReason = $state('user_request');
	let deleteDetail = $state('');
	let deleteTyped = $state('');
	let forceNeeded = $state(false);

	async function handleDelete(force = false) {
		deleteBusy = true;
		try {
			const qs = new URLSearchParams({ reason: deleteReason });
			if (deleteDetail) qs.set('detail', deleteDetail);
			if (force) qs.set('force', 'true');
			await api.delete(`games/id/${vm.entry.id}?${qs}`);
			await goto('/dashboard/trash');
		} catch (e) {
			if (String(e.message).includes('delegated by') && !force) {
				forceNeeded = true;
			} else {
				vm.ui.notice = e.message;
				vm.ui.noticeCritical = true;
				showDelete = false;
			}
		} finally {
			deleteBusy = false;
			if (!forceNeeded) showDelete = false;
		}
	}

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
				: 'https://example.com/embed-demo/'
	);

	function addRelatedGame() {
		vm.entry.relatedGames = [...(vm.entry.relatedGames ?? []), { id: '', title: '', slug: '' }];
	}

	function removeRelatedGame(index) {
		vm.entry.relatedGames = vm.entry.relatedGames.filter((_, i) => i !== index);
	}

	function onRelatedPick(index, gameId) {
		const picked = games.find((game) => String(game.id) === String(gameId));
		const next = [...vm.entry.relatedGames];
		next[index] = picked
			? { id: picked.id, title: picked.title, slug: picked.slug }
			: { id: '', title: '', slug: '' };
		vm.entry.relatedGames = next;
	}
</script>

{#snippet extraFields()}
	<div class="flex flex-col gap-1">
		<label class="text-sm font-medium text-dark/60" for="demo-type">Launcher type</label>
		<select
			id="demo-type"
			class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white disabled:opacity-60"
			value={vm.entry.demoType}
			onchange={(e) => vm.setDemoType(e.currentTarget.value)}
			disabled={!isOwner}
		>
			{#each GAME_DEMO_TYPES as type}
				<option value={type.value} disabled={type.disabled}>
					{type.label}{type.disabled ? ' (soon)' : ''}
				</option>
			{/each}
		</select>
	</div>
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
	{#if demoType === 'v86'}
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
						href="/dashboard/games/id/{data.id}/snapshot"
					>
						Open the snapshot studio →
					</a>
				</div>
			{/if}
		</div>
	{/if}
	{#if demoType === 'html5' || demoType === 'webgl' || demoType === 'jsdos' || demoType === 'v86'}
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
				{demoType === 'jsdos'
					? 'js-dos bundle'
					: demoType === 'v86'
						? 'v86 game ZIP'
						: demoType === 'html5'
							? 'HTML5 zip'
							: 'WebGL zip'}
			</label>
			<input
				id="demo-zip"
				type="file"
				accept={demoType === 'jsdos' ? '.jsdos' : '.zip,application/zip'}
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
		<label class="text-sm font-medium text-dark/60" for="instruction">Instruction</label>
		<textarea
			id="instruction"
			rows="3"
			class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors resize-none custom-scrollbar focus:bg-primary focus:text-white"
			placeholder="How to play…"
			readonly={!isOwner}
			bind:value={vm.entry.instruction}></textarea>
		<label class="text-sm font-medium text-dark/60" for="cheatcode">Cheat codes</label>
		<textarea
			id="cheatcode"
			rows="3"
			class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors resize-none custom-scrollbar focus:bg-primary focus:text-white"
			placeholder="Secrets, cheats, shortcuts…"
			readonly={!isOwner}
			bind:value={vm.entry.cheatcode}></textarea>
		<label class="text-sm font-medium text-dark/60" for="story">Story</label>
		<textarea
			id="story"
			rows="3"
			class="w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors resize-none custom-scrollbar focus:bg-primary focus:text-white"
			placeholder="The story so far…"
			readonly={!isOwner}
			bind:value={vm.entry.story}></textarea>
	</div>
	<div class="flex flex-col gap-2">
		<div class="flex items-center justify-between">
			<span class="text-sm font-medium text-dark/70">Related games</span>
			<button
				class="flex h-7 w-7 items-center justify-center rounded-lg border border-background bg-background/40 text-lg leading-none text-dark transition-colors hover:bg-background/60 disabled:opacity-50"
				disabled={!isOwner}
				onclick={addRelatedGame}
			>
				+
			</button>
		</div>
		{#each vm.entry.relatedGames as link, index}
			<div class="grid grid-cols-[1fr_auto] gap-1.5 items-center">
				<select
					class="w-full min-w-0 rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white disabled:opacity-60"
					value={String(link.id ?? '')}
					disabled={!isOwner}
					onchange={(e) => onRelatedPick(index, e.currentTarget.value)}
				>
					<option value="">Select a game…</option>
					{#each games as game}
						<option value={game.id}>{game.title}</option>
					{/each}
				</select>
				<button
					class="flex h-7 w-7 items-center justify-center rounded-lg border border-accent-red/30 bg-accent-red-light-4 text-accent-red transition-colors hover:bg-accent-red-light-3 disabled:opacity-50"
					disabled={!isOwner}
					onclick={() => removeRelatedGame(index)}
				>
					✕
				</button>
			</div>
		{/each}
	</div>
{/snippet}

<PostEditorShell
	{vm}
	kind="game"
	coverApiPath={`/api/games/id/${vm.entry.id}/cover`}
	titleLabel="Name"
	excerptRows={4}
	{extraFields}
/>

{#if mode === 'edit' && isOwner}
	<section class="rounded-xl border border-accent-red/30 bg-accent-red-light-4 p-4">
		<h3 class="font-semibold text-accent-red">Danger zone</h3>
		<p class="mt-1 text-sm text-dark/60">Delete this game. Projects delegating to it will show “Game unavailable” for 7 days.</p>
		<div class="mt-3 flex flex-wrap gap-2">
			<select bind:value={deleteReason} class="rounded-lg border border-dark/20 px-3 py-1 text-sm">
				<option value="user_request">User request</option>
				<option value="dmca">DMCA</option>
				<option value="moderation">Moderation</option>
				<option value="replaced">Replaced</option>
				<option value="other">Other</option>
			</select>
			<input bind:value={deleteDetail} placeholder="Detail (optional)" class="rounded-lg border border-dark/20 px-3 py-1 text-sm" />
			<button onclick={() => { forceNeeded = false; showDelete = true; }} class="rounded-full bg-accent-red px-4 py-2 text-sm font-medium text-white">Delete game</button>
		</div>
	</section>
	<ConfirmDialog
		open={showDelete && !forceNeeded}
		title="Delete game?"
		description="This will move the game to trash for 7 days. Delegated projects will keep their articles but lose the playable demo."
		confirmLabel="Delete"
		confirmColor="red"
		busy={deleteBusy}
		onconfirm={() => handleDelete(false)}
		oncancel={() => (showDelete = false)}
	/>
	<ConfirmDialog
		open={showDelete && forceNeeded}
		title="Game is delegated by published projects"
		description="Force delete will make those projects show “Game unavailable”. Type the game slug to confirm."
		confirmLabel="Force delete"
		confirmColor="red"
		requireTyping={data.slug}
		bind:typedValue={deleteTyped}
		busy={deleteBusy}
		onconfirm={() => handleDelete(true)}
		oncancel={() => { showDelete = false; forceNeeded = false; }}
	/>
{/if}
