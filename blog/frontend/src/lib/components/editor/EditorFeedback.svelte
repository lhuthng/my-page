<script>
	/**
	 * Renders the two message classes that need markup: sticky banners and
	 * transient toasts. (The third class, live, is inline status and belongs
	 * with the thing it describes — the slug beside its input, progress in the
	 * toolbar, the media count beside the media library.)
	 *
	 * Banners sit in normal flow below the header so a blocking problem pushes
	 * the body down rather than floating over it. Toasts float at the bottom
	 * right, so a save confirmation never reflows the editor.
	 *
	 * Both render a `×`. That is the invariant from docs/editor-feedback-ux.md,
	 * not a nicety: every message must have at least one exit — time,
	 * resolution, or an explicit dismiss.
	 */
	let { vm } = $props();

	const banners = $derived(vm.feedback.bannerList);
	const toasts = $derived(vm.feedback.toasts);

	const BANNER_TONE = {
		error: 'border-accent-red bg-accent-red-light-4',
		warning: 'border-accent-yellow bg-accent-yellow-light-4',
		info: 'border-accent-blue bg-accent-blue-light-4'
	};

	const TOAST_TONE = {
		success: 'border-accent-green bg-accent-green-light-3 text-accent-green-dark',
		neutral: 'border-dark/20 bg-white text-dark/70'
	};
</script>

{#if banners.length > 0}
	<div class="flex flex-col gap-2">
		{#each banners as banner (banner.id)}
			<div
				class="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-xl border-2 px-3 py-2 text-sm text-dark
					{BANNER_TONE[banner.tone] ?? BANNER_TONE.info}"
				role={banner.tone === 'error' ? 'alert' : 'status'}
			>
				<span class="min-w-0 grow">{banner.message}</span>
				{#each banner.actions as action (action.label)}
					<button class="font-medium text-accent-blue-dark underline" onclick={action.run}>
						{action.label}
					</button>
				{/each}
				<button
					class="shrink-0 rounded px-1.5 text-lg leading-none text-dark/50 transition-colors hover:text-dark"
					aria-label="Dismiss"
					onclick={() => vm.feedback.dismiss(banner.id)}
				>
					×
				</button>
			</div>
		{/each}
	</div>
{/if}

{#if toasts.length > 0}
	<div class="pointer-events-none fixed bottom-6 right-6 z-50 flex flex-col items-end gap-2">
		{#each toasts as toast (toast.id)}
			<div
				class="pointer-events-auto flex items-center gap-3 rounded-xl border-2 px-4 py-2 text-sm shadow-lg
					{TOAST_TONE[toast.tone] ?? TOAST_TONE.success}"
				role="status"
				onmouseenter={() => vm.feedback.pauseToast(toast.id)}
				onmouseleave={() => vm.feedback.resumeToast(toast.id)}
			>
				<span>{toast.message}</span>
				<button
					class="shrink-0 rounded px-1.5 text-lg leading-none opacity-60 transition-opacity hover:opacity-100"
					aria-label="Dismiss"
					onclick={() => vm.feedback.dismiss(toast.id)}
				>
					×
				</button>
			</div>
		{/each}
	</div>
{/if}
