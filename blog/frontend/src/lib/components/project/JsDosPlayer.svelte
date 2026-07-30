<script>
	import { onDestroy, onMount, tick } from 'svelte';
	import Portal from '$lib/components/shell/Portal.svelte';

	let { title, bundleUrl, beforeDemoPortal, afterDemoPortal } = $props();
	let shell = $state();
	let container = $state();
	let player;
	let overlayObserver;
	let layoutObserver;
	let normalLayoutTimer;
	let fullscreenStateTimer;
	let syncingFullscreenExit = false;
	let disposed = false;
	let fullscreenComboLatched = false;
	const pressedKeys = new Set();
	let mounted = $state(false);
	let fullscreenActive = $state(false);
	let state = $state('loading');
	let errorMessage = $state('');

	const isGameFullscreen = () =>
		!!document.fullscreenElement &&
		(document.fullscreenElement === shell || shell?.contains(document.fullscreenElement));

	const loadRuntime = () =>
		new Promise((resolve, reject) => {
			if (window.Dos) return resolve(window.Dos);
			const existing = document.querySelector('script[data-jsdos-runtime]');
			if (existing) {
				existing.addEventListener('load', () => resolve(window.Dos), { once: true });
				existing.addEventListener(
					'error',
					() => reject(new Error('js-dos runtime failed to load')),
					{
						once: true
					}
				);
				return;
			}
			const script = document.createElement('script');
			script.src = 'https://v8.js-dos.com/latest/js-dos.js';
			script.async = true;
			script.dataset.jsdosRuntime = 'true';
			script.onload = () =>
				window.Dos ? resolve(window.Dos) : reject(new Error('js-dos runtime unavailable'));
			script.onerror = () => reject(new Error('js-dos runtime failed to load'));
			document.head.appendChild(script);
		});

	const styleMouseCaptureOverlay = () => {
		const overlay = [...(container?.querySelectorAll('div') ?? [])].find(
			(element) =>
				element.textContent?.includes('Click to capture mouse') &&
				element.classList.contains('pointer-events-none')
		);
		if (overlay) {
			overlay.classList.add('jsdos-mouse-capture-overlay');
			overlay.style.setProperty('background', 'rgb(58 75 119 / 92%)', 'important');
		}
	};

	const toggleFullscreen = () => {
		if (!player) return;
		try {
			const entering = !isGameFullscreen();
			if (!entering) fullscreenActive = false;
			player.setFullScreen(entering);
			if (entering) {
				setTimeout(() => {
					if (!isGameFullscreen()) {
						fullscreenActive = false;
						scheduleNormalCanvasLayout();
					}
				}, 500);
			} else {
				scheduleNormalCanvasLayout();
			}
		} catch (error) {
			console.warn('Unable to toggle game fullscreen', error);
		}
	};

	const handleKeyDown = (event) => {
		if (event.code !== 'F8' && event.code !== 'F9') return;
		event.preventDefault();
		event.stopImmediatePropagation();
		pressedKeys.add(event.code);
		if (pressedKeys.has('F8') && pressedKeys.has('F9') && !fullscreenComboLatched) {
			fullscreenComboLatched = true;
			toggleFullscreen();
		}
	};

	const handleKeyUp = (event) => {
		if (event.code !== 'F8' && event.code !== 'F9') return;
		event.preventDefault();
		event.stopImmediatePropagation();
		pressedKeys.delete(event.code);
		fullscreenComboLatched = false;
	};

	const restoreNormalCanvasLayout = () => {
		if (disposed || isGameFullscreen() || !shell) return;
		fullscreenActive = false;
		const shellRect = shell.getBoundingClientRect();
		const width = shellRect.width;
		const height = shellRect.height;
		if (!width || !height) return;
		container?.querySelectorAll('canvas').forEach((canvas) => {
			canvas.style.position = 'relative';
			canvas.style.inset = 'auto';
			canvas.style.top = '0px';
			canvas.style.left = '0px';
			canvas.style.width = `${width}px`;
			canvas.style.height = `${height}px`;
			canvas.style.objectFit = 'fill';
		});
	};

	const scheduleNormalCanvasLayout = () => {
		clearInterval(normalLayoutTimer);
		normalLayoutTimer = setInterval(() => {
			restoreNormalCanvasLayout();
		}, 100);
		setTimeout(() => {
			clearInterval(normalLayoutTimer);
			normalLayoutTimer = undefined;
		}, 4000);
	};

	const handleFullscreenChange = () => {
		if (!document.fullscreenElement && player && fullscreenActive && !syncingFullscreenExit) {
			syncingFullscreenExit = true;
			try {
				player.setFullScreen(false);
			} catch {
				// js-dos may already have completed the exit transition.
			}
			setTimeout(() => (syncingFullscreenExit = false), 0);
		}
		fullscreenActive = isGameFullscreen();
		styleMouseCaptureOverlay();
		requestAnimationFrame(() => {
			window.dispatchEvent(new Event('resize'));
			if (!isGameFullscreen()) {
				scheduleNormalCanvasLayout();
			}
		});
	};

	onMount(async () => {
		mounted = true;
		await tick();
		if (disposed) return;
		window.addEventListener('keydown', handleKeyDown, true);
		window.addEventListener('keyup', handleKeyUp, true);
		document.addEventListener('fullscreenchange', handleFullscreenChange);
		fullscreenStateTimer = setInterval(() => {
			if (fullscreenActive && !isGameFullscreen()) {
				fullscreenActive = false;
				scheduleNormalCanvasLayout();
			}
		}, 250);
		if (!bundleUrl || !container) {
			state = 'error';
			errorMessage = 'The game bundle is unavailable.';
			return;
		}
		try {
			const Dos = await loadRuntime();
			if (disposed || !container) return;
			let options = {
				backend: 'dosboxX',
				workerThread: true,
				renderBackend: 'webgl',
				offscreenCanvas: true,
				imageRendering: 'smooth',
				renderAspect: 'Fit',
				kiosk: true,
				autoStart: true,
				mouseCapture: true,
				onEvent: (event) => {
					if (event === 'emu-ready' || event === 'bnd-play') state = 'ready';
					if (event === 'fullscreen-change') handleFullscreenChange();
				}
			};
			options.url = bundleUrl;
			player = Dos(container, options);
			styleMouseCaptureOverlay();
			requestAnimationFrame(styleMouseCaptureOverlay);
			overlayObserver = new MutationObserver(styleMouseCaptureOverlay);
			overlayObserver.observe(container, { childList: true, subtree: true });
			layoutObserver = new MutationObserver(() => {
				if (
					!isGameFullscreen() &&
					[...container.querySelectorAll('canvas')].some((canvas) => canvas.style.width === '0px')
				) {
					restoreNormalCanvasLayout();
				}
			});
			layoutObserver.observe(container, {
				attributes: true,
				attributeFilter: ['style'],
				subtree: true
			});
			state = 'ready';
		} catch (error) {
			state = 'error';
			errorMessage = error?.message ?? 'The game could not be loaded.';
		}
	});

	onDestroy(() => {
		disposed = true;
		overlayObserver?.disconnect();
		layoutObserver?.disconnect();
		clearInterval(normalLayoutTimer);
		clearInterval(fullscreenStateTimer);
		pressedKeys.clear();
		if (typeof window !== 'undefined') {
			window.removeEventListener('keydown', handleKeyDown, true);
			window.removeEventListener('keyup', handleKeyUp, true);
			document.removeEventListener('fullscreenchange', handleFullscreenChange);
			if (document.pointerLockElement && shell?.contains(document.pointerLockElement)) {
				document.exitPointerLock();
			}
		}
		// js-dos owns an emulator worker and audio pipeline; exit it before the
		// component is removed so navigation cannot leave sound behind.
		Promise.resolve(player?.stop?.()).catch(() => {
			// The runtime may already have stopped during a failed load.
		});
		container?.querySelectorAll('audio, video').forEach((media) => {
			media.pause();
			media.removeAttribute('src');
			media.load();
		});
		container?.replaceChildren();
		player = undefined;
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

{#if mounted}
	<div
		bind:this={shell}
		class="jsdos-shell"
		class:jsdos-fullscreen={fullscreenActive}
		aria-label={`${title} game`}
	>
		<div bind:this={container} class="jsdos-canvas"></div>
		{#if state === 'loading'}
			<div class="jsdos-status" role="status">Loading game…</div>
		{:else if state === 'error'}
			<div class="jsdos-status jsdos-error" role="alert">{errorMessage}</div>
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
