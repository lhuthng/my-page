<script>
	/**
	 * Floating message stack, bottom-right. Nothing here occupies layout, so a
	 * failure never pushes the page down — the pattern the editor established
	 * (`components/editor/EditorFeedback.svelte`, documented in
	 * `docs/editor-feedback-ux.md`): the page below never moves, and the
	 * message stays pinned while the content scrolls.
	 *
	 * The caller owns the lifetime: confirmations are pushed with a TTL,
	 * errors are left sticky until dismissed.
	 */
	import { fly } from 'svelte/transition';

	let { messages = [], ondismiss = () => {} } = $props();

	const TONE = {
		success: 'border-accent-green bg-accent-green-light-3 text-accent-green-dark',
		error: 'border-accent-red bg-accent-red-light-4 text-dark',
		info: 'border-accent-blue bg-accent-blue-light-4 text-dark'
	};
</script>

{#if messages.length > 0}
	<!-- Pointer events pass through the empty space around the cards, so an
	     invisible 24rem column never blocks the page underneath. -->
	<div
		class="pointer-events-none fixed right-4 bottom-6 z-50 flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-2"
	>
		{#each messages as message (message.id)}
			<div
				class="pointer-events-auto flex items-center gap-3 rounded-xl border-2 px-4 py-2 text-sm shadow-lg
					{TONE[message.tone] ?? TONE.info}"
				role={message.tone === 'error' ? 'alert' : 'status'}
				transition:fly={{ y: 8, duration: 150 }}
			>
				<span class="min-w-0 grow">{message.text}</span>
				<button
					class="shrink-0 rounded px-1.5 text-lg leading-none opacity-60 transition-opacity hover:opacity-100"
					aria-label="Dismiss"
					onclick={() => ondismiss(message.id)}
				>
					×
				</button>
			</div>
		{/each}
	</div>
{/if}
