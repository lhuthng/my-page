import { tick } from 'svelte';

export class JsDosPlayer {
	shell = $state();
	container = $state();
	mounted = $state(false);
	fullscreenActive = $state(false);
	state = $state('loading');
	errorMessage = $state('');

	#player;
	#overlayObserver;
	#layoutObserver;
	#normalLayoutTimer;
	#fullscreenStateTimer;
	#syncingFullscreenExit = false;
	#disposed = false;
	#fullscreenComboLatched = false;
	#pressedKeys = new Set();

	constructor({ bundleUrl }) {
		this.bundleUrl = bundleUrl;
	}

	isGameFullscreen = () =>
		!!document.fullscreenElement &&
		(document.fullscreenElement === this.shell || this.shell?.contains(document.fullscreenElement));

	loadRuntime = () =>
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

	styleMouseCaptureOverlay = () => {
		const overlay = [...(this.container?.querySelectorAll('div') ?? [])].find(
			(element) =>
				element.textContent?.includes('Click to capture mouse') &&
				element.classList.contains('pointer-events-none')
		);
		if (overlay) {
			overlay.classList.add('jsdos-mouse-capture-overlay');
			overlay.style.setProperty('background', 'rgb(58 75 119 / 92%)', 'important');
		}
	};

	toggleFullscreen = () => {
		if (!this.#player) return;
		try {
			const entering = !this.isGameFullscreen();
			if (!entering) this.fullscreenActive = false;
			this.#player.setFullScreen(entering);
			if (entering) {
				setTimeout(() => {
					if (!this.isGameFullscreen()) {
						this.fullscreenActive = false;
						this.scheduleNormalCanvasLayout();
					}
				}, 500);
			} else {
				this.scheduleNormalCanvasLayout();
			}
		} catch (error) {
			console.warn('Unable to toggle game fullscreen', error);
		}
	};

	handleKeyDown = (event) => {
		if (event.code !== 'F8' && event.code !== 'F9') return;
		event.preventDefault();
		event.stopImmediatePropagation();
		this.#pressedKeys.add(event.code);
		if (
			this.#pressedKeys.has('F8') &&
			this.#pressedKeys.has('F9') &&
			!this.#fullscreenComboLatched
		) {
			this.#fullscreenComboLatched = true;
			this.toggleFullscreen();
		}
	};

	handleKeyUp = (event) => {
		if (event.code !== 'F8' && event.code !== 'F9') return;
		event.preventDefault();
		event.stopImmediatePropagation();
		this.#pressedKeys.delete(event.code);
		this.#fullscreenComboLatched = false;
	};

	restoreNormalCanvasLayout = () => {
		if (this.#disposed || this.isGameFullscreen() || !this.shell) return;
		this.fullscreenActive = false;
		const shellRect = this.shell.getBoundingClientRect();
		const width = shellRect.width;
		const height = shellRect.height;
		if (!width || !height) return;
		this.container?.querySelectorAll('canvas').forEach((canvas) => {
			canvas.style.position = 'relative';
			canvas.style.inset = 'auto';
			canvas.style.top = '0px';
			canvas.style.left = '0px';
			canvas.style.width = `${width}px`;
			canvas.style.height = `${height}px`;
			canvas.style.objectFit = 'fill';
		});
	};

	scheduleNormalCanvasLayout = () => {
		clearInterval(this.#normalLayoutTimer);
		this.#normalLayoutTimer = setInterval(() => {
			this.restoreNormalCanvasLayout();
		}, 100);
		setTimeout(() => {
			clearInterval(this.#normalLayoutTimer);
			this.#normalLayoutTimer = undefined;
		}, 4000);
	};

	handleFullscreenChange = () => {
		if (
			!document.fullscreenElement &&
			this.#player &&
			this.fullscreenActive &&
			!this.#syncingFullscreenExit
		) {
			this.#syncingFullscreenExit = true;
			try {
				this.#player.setFullScreen(false);
			} catch {
				// js-dos may already have completed the exit transition.
			}
			setTimeout(() => (this.#syncingFullscreenExit = false), 0);
		}
		this.fullscreenActive = this.isGameFullscreen();
		this.styleMouseCaptureOverlay();
		requestAnimationFrame(() => {
			window.dispatchEvent(new Event('resize'));
			if (!this.isGameFullscreen()) {
				this.scheduleNormalCanvasLayout();
			}
		});
	};

	async mount() {
		this.mounted = true;
		await tick();
		if (this.#disposed) return;
		window.addEventListener('keydown', this.handleKeyDown, true);
		window.addEventListener('keyup', this.handleKeyUp, true);
		document.addEventListener('fullscreenchange', this.handleFullscreenChange);
		this.#fullscreenStateTimer = setInterval(() => {
			if (this.fullscreenActive && !this.isGameFullscreen()) {
				this.fullscreenActive = false;
				this.scheduleNormalCanvasLayout();
			}
		}, 250);
		if (!this.bundleUrl || !this.container) {
			this.state = 'error';
			this.errorMessage = 'The game bundle is unavailable.';
			return;
		}
		try {
			const Dos = await this.loadRuntime();
			if (this.#disposed || !this.container) return;
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
					if (event === 'emu-ready' || event === 'bnd-play') this.state = 'ready';
					if (event === 'fullscreen-change') this.handleFullscreenChange();
				}
			};
			options.url = this.bundleUrl;
			this.#player = Dos(this.container, options);
			this.styleMouseCaptureOverlay();
			requestAnimationFrame(this.styleMouseCaptureOverlay);
			this.#overlayObserver = new MutationObserver(this.styleMouseCaptureOverlay);
			this.#overlayObserver.observe(this.container, { childList: true, subtree: true });
			this.#layoutObserver = new MutationObserver(() => {
				if (
					!this.isGameFullscreen() &&
					[...this.container.querySelectorAll('canvas')].some(
						(canvas) => canvas.style.width === '0px'
					)
				) {
					this.restoreNormalCanvasLayout();
				}
			});
			this.#layoutObserver.observe(this.container, {
				attributes: true,
				attributeFilter: ['style'],
				subtree: true
			});
			this.state = 'ready';
		} catch (error) {
			this.state = 'error';
			this.errorMessage = error?.message ?? 'The game could not be loaded.';
		}
	}

	unmount() {
		this.#disposed = true;
		this.#overlayObserver?.disconnect();
		this.#layoutObserver?.disconnect();
		clearInterval(this.#normalLayoutTimer);
		clearInterval(this.#fullscreenStateTimer);
		this.#pressedKeys.clear();
		if (typeof window !== 'undefined') {
			window.removeEventListener('keydown', this.handleKeyDown, true);
			window.removeEventListener('keyup', this.handleKeyUp, true);
			document.removeEventListener('fullscreenchange', this.handleFullscreenChange);
			if (document.pointerLockElement && this.shell?.contains(document.pointerLockElement)) {
				document.exitPointerLock();
			}
		}
		// js-dos owns an emulator worker and audio pipeline; exit it before the
		// component is removed so navigation cannot leave sound behind.
		Promise.resolve(this.#player?.stop?.()).catch(() => {
			// The runtime may already have stopped during a failed load.
		});
		this.container?.querySelectorAll('audio, video').forEach((media) => {
			media.pause();
			media.removeAttribute('src');
			media.load();
		});
		this.container?.replaceChildren();
		this.#player = undefined;
	}
}
