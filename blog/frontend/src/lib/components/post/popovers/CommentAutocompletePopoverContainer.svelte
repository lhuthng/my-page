<script>
	import { fade } from 'svelte/transition';

	let { open = false, top = null, className = '', children } = $props();

	let popoverEl = $state(null);
	let displayTop = $state(null);
	let previousHeight = 0;

	$effect(() => {
		if (top != null) {
			displayTop = top;
			return;
		}

		if (!open) {
			displayTop = null;
		}
	});

	const popoverStyle = $derived(
		displayTop == null ? 'visibility: hidden;' : `top: ${displayTop}px;`
	);

	$effect.pre(() => {
		if (popoverEl) {
			previousHeight = popoverEl.offsetHeight;
		} else {
			previousHeight = 0;
		}
	});

	$effect(() => {
		if (!popoverEl) return;

		const nextHeight = popoverEl.offsetHeight;

		if (previousHeight > 0 && nextHeight > 0 && previousHeight !== nextHeight) {
			popoverEl.animate([{ height: `${previousHeight}px` }, { height: `${nextHeight}px` }], {
				duration: 250,
				easing: 'ease-out'
			});
		} else if (previousHeight === 0 && nextHeight > 0) {
			popoverEl.animate(
				[
					{ opacity: 0, transform: 'scale(0.95) translateY(10px)' },
					{ opacity: 1, transform: 'scale(1) translateY(0)' }
				],
				{
					duration: 200,
					easing: 'ease-out'
				}
			);
		}
	});
</script>

{#if open}
	<div
		bind:this={popoverEl}
		class={`comment-autocomplete-popover absolute left-2 right-2 z-50 overflow-hidden rounded-xl border-2 border-primary bg-white py-2 shadow-lg ${className}`}
		style={popoverStyle}
		onmousedown={(event) => {
			event.preventDefault();
		}}
		role="listbox"
		aria-label="Suggestions"
		tabindex="-1"
	>
		{@render children?.()}
	</div>
{/if}
