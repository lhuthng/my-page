<script>
	/**
	 * The editor's sticky header: identity (title/slug), save status, and the
	 * primary actions. Always visible — the old toolbar lived in a drawer that
	 * had to be expanded to see status or reach Save/Publish at all.
	 *
	 * It owns the *live* feedback for the fields it renders: the slug
	 * availability marker, validation errors pinned under the input that caused
	 * them, and the upload/build progress line. Sticky banners and toasts are
	 * rendered by `EditorFeedback` further down, so this stays one row of
	 * chrome plus a fixed-height status line that never shifts the body.
	 */
	let { vm, titleLabel = 'Title' } = $props();

	const entry = $derived(vm.entry);
	const ui = $derived(vm.ui);
	const live = $derived(vm.feedback.liveSlots);

	const slugState = $derived(live.slug);
	const saving = $derived(ui.save.status === 'saving');
</script>

<header
	class="sticky top-0 z-20 bg-white/95 backdrop-blur border-b border-dark/10 shadow-sm rounded-xl"
>
	<div class="flex flex-wrap items-center gap-x-4 gap-y-2 px-4 lg:px-6 pt-3 pb-1">
		<div class="flex min-w-0 grow flex-col gap-1">
			<input
				id="editor-title"
				class="w-full min-w-0 rounded-xl bg-transparent px-2 py-1 text-2xl font-bold text-dark outline-none border-dark border-2 transition-colors focus:bg-primary focus:text-white
					{ui.fieldErrors.title ? 'border-accent-red' : ''}"
				placeholder={titleLabel}
				bind:value={entry.title}
				oninput={() => vm.clearFieldError('title')}
				autocomplete="off"
				readonly={!vm.isOwner}
				required
			/>
			{#if ui.fieldErrors.title}
				<p class="pl-2 text-sm font-medium text-accent-red">{ui.fieldErrors.title}</p>
			{/if}

			<div class="flex min-w-0 items-center gap-4 pl-2">
				<label class="shrink-0 text-sm text-dark/40" for="editor-slug">Slug/</label>
				<input
					id="editor-slug"
					class="min-w-0 w-52 rounded-lg bg-transparent px-2 py-1 text-sm text-dark outline-none border-dark border-2 transition-colors focus:bg-primary focus:text-white
						{ui.fieldErrors.slug ? 'border-accent-red' : ''}"
					bind:value={entry.slug}
					oninput={() => vm.clearFieldError('slug')}
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
			{#if ui.fieldErrors.slug}
				<p class="pl-2 text-sm font-medium text-accent-red">{ui.fieldErrors.slug}</p>
			{/if}
		</div>

		<div class="min-w-60 flex flex-col items-end gap-3">
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

	<!--
		Live status row. `min-h-6` reserves the line whether or not anything is
		happening, so the body below never jumps when progress starts or ends —
		the old `{#if ui.progress}` block added and removed a whole line.
		`aria-atomic="false"` so only the changed fragment is announced.
	-->
	<div
		class="flex min-h-6 items-center gap-2 px-4 lg:px-6 pb-2"
		aria-live="polite"
		aria-atomic="false"
	>
		{#if live.progress}
			<p class="text-sm text-dark/70">{live.progress}</p>
		{/if}
	</div>
</header>
