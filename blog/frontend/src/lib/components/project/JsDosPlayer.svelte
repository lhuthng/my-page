<script>
	import { onMount } from 'svelte';
	import Portal from '$lib/components/shell/Portal.svelte';
	import { JsDosPlayer } from '$lib/players/JsDosPlayer.svelte.js';

	let { title, bundleUrl, beforeDemoPortal, afterDemoPortal } = $props();
	const player = new JsDosPlayer({ bundleUrl });

	onMount(() => {
		player.mount();
		return () => player.unmount();
	});
</script>

<Portal target={afterDemoPortal} class="text-dark/75 text-sm">
	Click the game to capture the mouse. Press <kbd>Esc</kbd>
	to release it. Press
	<kbd>F8</kbd>
	+
	<kbd>F9</kbd>
	together to toggle fullscreen.
</Portal>

{#if player.mounted}
	<div
		bind:this={player.shell}
		class="jsdos-shell"
		class:jsdos-fullscreen={player.fullscreenActive}
		aria-label={`${title} game`}
	>
		<div bind:this={player.container} class="jsdos-canvas"></div>
		{#if player.state === 'loading'}
			<div class="jsdos-status" role="status">Loading game…</div>
		{:else if player.state === 'error'}
			<div class="jsdos-status jsdos-error" role="alert">{player.errorMessage}</div>
		{/if}
	</div>
{/if}

<style>
	.jsdos-shell {
		position: relative;
		width: 100%;
		height: 100%;
		max-width: 100%;
		overflow: hidden;
		background: #000;
	}

	.jsdos-shell:fullscreen {
		width: 100vw;
		height: 100vh;
		max-width: none;
		border-radius: 0;
	}

	/* js-dos normally gets these fullscreen dimensions from its global utility
	 * stylesheet. Recreate only the required fullscreen behavior locally. */
	:global(.jsdos-canvas:fullscreen),
	:global(.jsdos-canvas :fullscreen) {
		width: 100vw !important;
		height: 100vh !important;
		overflow: hidden !important;
		background: #000 !important;
	}

	:global(.jsdos-canvas:fullscreen canvas),
	:global(.jsdos-canvas :fullscreen canvas) {
		position: fixed !important;
		inset: 0 !important;
		width: 100vw !important;
		height: 100vh !important;
		object-fit: contain;
		background: #000;
		image-rendering: auto !important;
	}

	:global(.jsdos-canvas:fullscreen .jsdos-mouse-capture-overlay),
	:global(.jsdos-canvas :fullscreen .jsdos-mouse-capture-overlay) {
		position: fixed !important;
		inset: 0 !important;
		width: 100vw !important;
		height: 100vh !important;
	}

	/* Restore the normal player geometry after js-dos returns from fullscreen. */
	:global(.jsdos-canvas > div:first-child),
	:global(.jsdos-canvas .window),
	:global(.jsdos-canvas .window > .relative),
	:global(.jsdos-canvas .bg-black) {
		width: 100% !important;
		height: 100% !important;
	}

	.jsdos-shell:not(.jsdos-fullscreen) :global(.jsdos-canvas canvas) {
		position: relative !important;
		inset: auto !important;
		top: 0 !important;
		left: 0 !important;
		width: 100% !important;
		height: 100% !important;
		object-fit: contain !important;
	}

	:global(.jsdos-canvas canvas[style*='width: 0px']) {
		position: relative !important;
		inset: auto !important;
		top: 0 !important;
		left: 0 !important;
		width: 100% !important;
		height: 100% !important;
		object-fit: contain !important;
	}

	/* Browser truth wins if js-dos leaves its own fullscreen class stale. */
	:global(body:not(:has(:fullscreen))) .jsdos-shell :global(.jsdos-canvas canvas) {
		position: relative !important;
		inset: auto !important;
		top: 0 !important;
		left: 0 !important;
		width: 100% !important;
		height: 100% !important;
		object-fit: contain !important;
	}

	.jsdos-canvas {
		width: 100%;
		height: 100%;
	}

	.jsdos-status {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		color: #fff;
		background: #000;
		font:
			500 1rem/1.5 system-ui,
			sans-serif;
	}

	.jsdos-error {
		padding: 2rem;
		text-align: center;
		color: #ffb4ab;
	}

	/* js-dos creates this capture prompt inside its own player tree. Keep the
	 * override limited to that tree; no js-dos stylesheet is loaded globally. */
	:global(.jsdos-mouse-capture-overlay) {
		display: grid;
		place-items: center;
		background: rgb(58 75 119 / 92%) !important;
		color: #fff;
		font-family: inherit;
		font-size: clamp(0.9rem, 2vw, 1.25rem);
		font-weight: 600;
		line-height: 1.5;
		text-align: center;
		text-shadow: 0 1px 2px rgb(31 41 68 / 80%);
		z-index: 10;
	}
</style>
