<script>
	/**
	 * The editor's sticky header: identity (title/slug), save status, the
	 * draft/published toggle, and the primary actions. Always visible — the
	 * old toolbar lived in a drawer that had to be expanded to see status or
	 * reach Save/Publish at all, and its status text was a 2-second toast that
	 * was invisible whenever the drawer was collapsed.
	 */
	let { vm, titleLabel = 'Title' } = $props();

	const entry = $derived(vm.entry);
	const ui = $derived(vm.ui);

	const slugState = $derived(ui.slugStatus[entry.slug]);
	const saving = $derived(ui.save.status === 'saving');
</script>

<header
	class="sticky top-0 z-20 bg-white/95 backdrop-blur border-b border-dark/10 shadow-sm rounded-xl"
>
	<div class="flex flex-wrap items-center gap-x-4 gap-y-2 px-4 lg:px-6 pt-3 pb-1">
		<div class="flex min-w-0 grow flex-col gap-1">
			<input
				id="editor-title"
				class="w-full min-w-0 rounded-xl bg-transparent px-2 py-1 text-2xl font-bold text-dark outline-none border-dark border-2 transition-colors focus:bg-primary focus:text-white"
				placeholder={titleLabel}
				bind:value={entry.title}
				autocomplete="off"
				readonly={!vm.isOwner}
				required
			/>
			<div class="flex min-w-0 items-center gap-4 pl-2">
				<label class="shrink-0 text-sm text-dark/40" for="editor-slug">Slug/</label>
				<input
					id="editor-slug"
					class="min-w-0 w-52 rounded-lg bg-transparent px-2 py-1 text-sm text-dark outline-none border-dark border-2 transition-colors focus:bg-primary focus:text-white"
					bind:value={entry.slug}
					autocomplete="off"
					readonly={!vm.isOwner}
					required
				/>
				{#if slugState === 'used'}
					<span class="text-sm font-medium text-accent-red">taken</span>
				{:else if slugState === 'ready'}
					<span class="text-sm font-medium text-accent-green">ok</span>
				{:else if slugState === 'pending'}
					<span class="text-sm font-medium text-accent-yellow">checking…</span>
				{/if}
			</div>
		</div>

		<div class="min-w-60 flex flex-col items-end gap-3">
			{#if vm.mode === 'edit' && vm.isOwner}
				<div class="ml-auto duo-btn w-fit" data-duo-color="blue">
					<button onclick={vm.toggleVersion}>
						Ver. {vm.forDraft ? 'Draft' : 'Published'}
					</button>
				</div>
			{/if}

			<div class="flex gap-2 items-center">
				<span
					class="inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-sm font-medium
					{vm.isDirty
						? 'bg-accent-red-light-3 text-accent-red-dark'
						: 'bg-accent-green-light-3 text-accent-green-dark'}"
					title={vm.isDirty ? 'Unsaved changes' : 'No unsaved changes'}
				>
					<span
						class="inline-block w-2 h-2 rounded-full"
						class:bg-accent-red={vm.isDirty}
						class:bg-accent-green={!vm.isDirty}
					></span>
					{vm.isDirty ? 'Unsaved' : 'Saved'}
				</span>

				{#if vm.mode === 'create'}
					<div class="duo-btn" data-duo-color="green">
						<button disabled={saving} onclick={vm.submit}>
							{saving ? 'Saving…' : 'Submit'}
						</button>
					</div>
				{:else if !vm.isOwner}
					<span class="text-sm italic text-dark/50">View only</span>
				{:else}
					<div class="duo-btn" data-duo-color="green">
						<button disabled={saving} onclick={vm.save}>
							{saving ? 'Saving…' : 'Save'}
						</button>
					</div>
					<div class="duo-btn" data-duo-color="green">
						<button disabled={ui.isPublishing} onclick={vm.publish}>
							{ui.isPublishing ? 'Publishing…' : 'Publish'}
						</button>
					</div>
				{/if}
			</div>
		</div>
	</div>

	<div class="flex flex-col gap-1 px-4 lg:px-6 pb-2">
		{#if ui.progress}
			<p class="text-sm text-dark/70">{ui.progress}</p>
		{/if}

		{#if ui.notice}
			<p class:text-accent-green={!ui.noticeCritical} class:text-accent-red={ui.noticeCritical}>
				{ui.notice}
			</p>
		{/if}
	</div>

	{#if ui.save.status === 'conflict'}
		<div
			class="mx-4 lg:mx-6 mb-2 flex flex-wrap items-center gap-2 rounded-xl border-2 border-accent-yellow bg-accent-yellow-light-4 px-3 py-2 text-sm text-dark"
		>
			<span>Someone else saved this {vm.kind} while you were editing.</span>
			<button class="font-medium text-accent-blue-dark underline" onclick={vm.acceptRemoteVersion}>
				Reload their version
			</button>
			<span>or</span>
			<button
				class="font-medium text-accent-blue-dark underline"
				onclick={vm.overwriteRemoteVersion}
			>
				overwrite with mine
			</button>
		</div>
	{/if}

	{#if vm.localDraftAvailable}
		<div
			class="mx-4 lg:mx-6 mb-2 flex flex-wrap items-center gap-2 rounded-xl border-2 border-accent-blue bg-accent-blue-light-4 px-3 py-2 text-sm text-dark"
		>
			<span>A locally-saved draft is newer than what's shown here.</span>
			<button class="font-medium text-accent-blue-dark underline" onclick={vm.recoverLocalDraft}>
				Restore it
			</button>
			<span>or</span>
			<button class="font-medium text-accent-blue-dark underline" onclick={vm.discardLocalDraft}>
				discard it
			</button>
		</div>
	{/if}
</header>
