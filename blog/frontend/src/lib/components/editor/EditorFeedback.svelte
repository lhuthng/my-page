<script>
	/**
	 * Renders the two message classes that need markup: sticky banners and
	 * transient toasts. (The third class, live, is inline status and belongs
	 * with the thing it describes — the slug beside its input, progress in the
	 * toolbar, the media count beside the media library.)
	 *
	 * Both classes render in **one floating stack in the bottom-right corner**.
	 * This replaced an in-flow banner block below the header, which pushed the
	 * body down the moment anything went wrong: the editor reflowed under the
	 * reader's cursor at exactly the wrong time. Nothing here occupies layout
	 * — the page below never moves, and the messages stay pinned while the
	 * article scrolls, which is also what keeps a failure on screen long
	 * enough to act on.
	 *
	 * Bottom-right, not top-right, because the top-right of this page is
	 * already taken: the site header is `fixed` across the full width there,
	 * and the editor's own sticky toolbar puts Save/Publish at the right edge
	 * of the content column. A popup in that corner would sit on the buttons
	 * it is telling you about. The stack lifts above the mini audiobook player
	 * the same way `ToTop` does.
	 *
	 * Both classes render a `×`. That is the invariant from
	 * docs/editor-feedback-ux.md, not a nicety: every message must have at
	 * least one exit — time, resolution, or an explicit dismiss.
	 */
	import { audiobookSession } from '$lib/players/AudiobookSession.svelte.js';

	let { vm } = $props();

	const banners = $derived(vm.feedback.bannerList);
	const toasts = $derived(vm.feedback.toasts);
	const anyMessage = $derived(banners.length > 0 || toasts.length > 0);

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

{#if anyMessage}
	<!-- Pointer-events pass through the empty space around the cards, so an
	     invisible 24rem column never blocks the page underneath. -->
	<div
		class="pointer-events-none fixed right-4 z-50 flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-2
			{audiobookSession.miniVisible ? 'bottom-35' : 'bottom-6'}"
	>
		<!-- Sticky, then transient: a problem that needs a decision outranks an
		     acknowledgement, so it sits closest to the corner the eye lands on. -->
		{#each banners as banner (banner.id)}
			<div
				class="pointer-events-auto flex flex-wrap items-center gap-x-3 gap-y-1 rounded-xl border-2 px-3 py-2 text-sm text-dark shadow-lg
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
