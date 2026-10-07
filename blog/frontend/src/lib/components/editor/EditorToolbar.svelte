<script>
	/**
	 * The editor's sticky header: identity (title/slug), save status, and the
	 * primary actions. Always visible — the old toolbar lived in a drawer that
	 * had to be expanded to see status or reach Save/Publish at all.
	 *
	 * It owns the *live* feedback for the fields it renders: the slug
	 * availability marker, validation errors pinned under the input that caused
	 * them, the save-status pill, and the upload/build progress line. Sticky
	 * banners and toasts are rendered by `EditorFeedback`, which floats in a
	 * corner, so this stays one row of chrome plus a fixed-height status line
	 * that never shifts the body.
	 */
	import { saveIndicator } from '$lib/features/editor/model/save-status.js';

	let { vm, titleLabel = 'Title' } = $props();

	const entry = $derived(vm.entry);
	const ui = $derived(vm.ui);
	const live = $derived(vm.feedback.liveSlots);

	const slugState = $derived(live.slug);
	const saving = $derived(ui.save.status === 'saving');

	// The pill reports the *save lifecycle*, not just the dirty flag: a save or
	// publish in flight, a failure, a conflict. `null` in a fresh create form,
	// where there is no save to describe yet.
	const indicator = $derived(
		saveIndicator({
			mode: vm.mode,
			status: ui.save.status,
			isDirty: vm.isDirty,
			isPublishing: ui.isPublishing
		})
	);

	const PILL_TONE = {
		busy: 'bg-accent-blue-light-3 text-accent-blue-dark',
		dirty: 'bg-accent-yellow-light-3 text-accent-yellow-dark',
		error: 'bg-accent-red-light-3 text-accent-red-dark',
		saved: 'bg-accent-green-light-3 text-accent-green-dark'
	};

	const DOT_TONE = {
		busy: 'bg-accent-blue',
		dirty: 'bg-accent-yellow',
		error: 'bg-accent-red',
		saved: 'bg-accent-green'
	};
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
				{#if indicator}
					<span
						class="inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-sm font-medium
							{PILL_TONE[indicator.tone]}"
						title={indicator.title}
						aria-live="polite"
					>
						<span class="inline-block w-2 h-2 rounded-full {DOT_TONE[indicator.tone]}"></span>
						{indicator.label}
					</span>
				{/if}

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
