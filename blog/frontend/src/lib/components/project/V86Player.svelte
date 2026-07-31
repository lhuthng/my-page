<script>
	import { onMount } from 'svelte';
	import Portal from '$lib/components/shell/Portal.svelte';

	let { title, runtime, beforeDemoPortal, afterDemoPortal } = $props();
	let screenContainer;
	let shell;
	let emulator;
	let status = $state('Loading emulator…');
	let error = $state('');
	let running = $state(false);
	let pressed = new Set();
	let fullscreenComboLatched = false;
	let disposed = false;
	let mouseRemainderX = 0;
	let mouseRemainderY = 0;
	let mouseSensitivity = $state(0.4);
	let noiseReductionStrength = $state(200);
	let disableMouseWheel = $state(true);
	let noiseChain = null;
	let noiseChainPromise = null;

	const loadRuntime = () =>
		new Promise((resolve, reject) => {
			if (window.V86 || window.V86Starter) return resolve();
			const existing = document.querySelector('script[data-v86-runtime]');
			if (existing) {
				existing.addEventListener('load', resolve, { once: true });
				existing.addEventListener('error', reject, { once: true });
				return;
			}
			const script = document.createElement('script');
			script.src = '/v86/build/libv86.js';
			script.dataset.v86Runtime = 'true';
			script.onload = resolve;
			script.onerror = () => reject(new Error('The v86 runtime could not be loaded.'));
			document.head.append(script);
		});

	const start = async () => {
		if (running || !runtime) return;
		error = '';
		status = 'Loading emulator…';
		try {
			if ('serviceWorker' in navigator) {
				await navigator.serviceWorker.register('/v86-cache-worker.js', { scope: '/' });
				await navigator.serviceWorker.ready;
			}
			await loadRuntime();
			if (disposed) return;
			const V86Constructor = window.V86 ?? window.V86Starter;
			if (!V86Constructor) throw new Error('The local v86 constructor is unavailable.');
			emulator = new V86Constructor({
				wasm_path: '/v86/build/v86.wasm',
				screen: {
					container: screenContainer,
					use_graphical_text: false
				},
				screen_container: screenContainer,
				autostart: true,
				memory_size: runtime.memory_size,
				vga_memory_size: runtime.vga_memory_size,
				boot_order: 0,
				bios: { url: '/v86/bios/seabios.bin' },
				vga_bios: { url: '/v86/bios/vgabios.bin' },
				hda: {
					url: runtime.base_url,
					size: runtime.base_size_bytes,
					async: true,
					fixed_chunk_size: runtime.chunk_size_bytes,
					use_parts: true
				},
				cdrom: {
					url: runtime.iso_url,
					size: runtime.iso_size_bytes,
					async: false
				},
				acpi: false,
				disable_speaker: false,
				net_device: {
					type: 'ne2k',
					relay_url: undefined,
					cors_proxy: undefined,
					mtu: 1500
				},
				filesystem: {}
			});
			if (disposed) {
				destroy();
				return;
			}
			running = true;
			status = `Booting ${runtime.system_name}…`;
			emulator.add_listener?.('emulator-ready', () => {
				status = 'Running';
				applyNoiseFilter().catch(() => {});
			});
			emulator.add_listener?.('download-progress', (info) => {
				if (info?.file_name !== runtime.iso_url) return;
				const loaded = Math.floor((info.loaded ?? 0) / 1048576);
				const total = Math.floor((info.total ?? 0) / 1048576);
				if (info.loaded != null && info.total != null && info.loaded >= info.total) {
					status = `Booting ${runtime.system_name}…`;
				} else {
					status = `Downloading ISO (${loaded} / ${total} MB)`;
				}
			});
			applyNoiseFilter().catch(() => {});
		} catch (cause) {
			error = cause?.message ?? 'v86 failed to start.';
			status = '';
			running = false;
		}
	};

	const captureMouse = () => {
		if (typeof emulator?.lock_mouse === 'function') emulator.lock_mouse();
		else screenContainer?.querySelector('canvas')?.requestPointerLock?.();
	};

	const toggleFullscreen = async () => {
		if (!document.fullscreenElement) {
			await shell?.requestFullscreen?.();
		} else {
			await document.exitFullscreen?.();
		}
	};

	const destroy = () => {
		try {
			emulator?.stop?.();
			emulator?.destroy?.();
		} finally {
			emulator = undefined;
			running = false;
			for (const context of document.querySelectorAll('audio')) {
				if (shell?.contains(context)) context.remove();
			}
		}
	};

	const createNoiseGateWorkletUrl = () => {
		const source = `
			class NoiseGateProcessor extends AudioWorkletProcessor {
				constructor() {
					super();
					this.envelope = 0;
					this.threshold = 0.008;
					this.attack = 0.004;
					this.release = 0.18;
					this.port.onmessage = (event) => {
						if (event.data?.type === 'threshold') this.threshold = event.data.value;
					};
				}
				process(inputs, outputs) {
					const input = inputs[0];
					const output = outputs[0];
					if (!input || !input[0] || !output || !output[0]) return true;
					const inL = input[0];
					const inR = input[1] || inL;
					const outL = output[0];
					const outR = output[1] || outL;
					const n = inL.length;
					const atk = 1 - Math.exp(-1 / (this.attack * sampleRate));
					const rel = 1 - Math.exp(-1 / (this.release * sampleRate));
					for (let i = 0; i < n; i++) {
						const peak = Math.abs(inL[i]) > Math.abs(inR[i]) ? Math.abs(inL[i]) : Math.abs(inR[i]);
						const target = peak > this.threshold ? 1 : 0;
						this.envelope =
							target > this.envelope
								? this.envelope + (1 - this.envelope) * atk
								: this.envelope * (1 - rel);
						outL[i] = inL[i] * this.envelope;
						outR[i] = inR[i] * this.envelope;
					}
					return true;
				}
			}
			registerProcessor('v86-noise-gate', NoiseGateProcessor);
		`;
		return URL.createObjectURL(new Blob([source], { type: 'application/javascript' }));
	};

	const buildNoiseChain = async (ctx) => {
		let lowpass;
		try {
			lowpass = ctx.createBiquadFilter();
			lowpass.type = 'lowpass';
			lowpass.frequency.value = 8000;
			lowpass.Q.value = 0.7;
		} catch {
			return null;
		}
		let gate = null;
		try {
			if (ctx.audioWorklet?.addModule) {
				const url = createNoiseGateWorkletUrl();
				await Promise.race([
					ctx.audioWorklet.addModule(url),
					new Promise((_, reject) =>
						setTimeout(() => reject(new Error('worklet load timed out')), 1500)
					)
				]);
				URL.revokeObjectURL(url);
				gate = new AudioWorkletNode(ctx, 'v86-noise-gate', {
					numberOfInputs: 1,
					numberOfOutputs: 1,
					outputChannelCount: [2]
				});
			}
		} catch (cause) {
			console.warn('v86 noise-gate worklet unavailable, falling back.', cause);
		}
		if (!gate && ctx.createScriptProcessor) {
			gate = ctx.createScriptProcessor(4096, 2, 2);
			gate.threshold = 0.008;
			let envelope = 0;
			const atk = 1 - Math.exp(-1 / (0.004 * ctx.sampleRate));
			const rel = 1 - Math.exp(-1 / (0.18 * ctx.sampleRate));
			gate.onaudioprocess = function (event) {
				const inL = event.inputBuffer.getChannelData(0);
				const inR = event.inputBuffer.getChannelData(1);
				const outL = event.outputBuffer.getChannelData(0);
				const outR = event.outputBuffer.getChannelData(1);
				const threshold = this.threshold;
				for (let i = 0; i < inL.length; i++) {
					const peak = Math.abs(inL[i]) > Math.abs(inR[i]) ? Math.abs(inL[i]) : Math.abs(inR[i]);
					const target = peak > threshold ? 1 : 0;
					envelope = target > envelope ? envelope + (1 - envelope) * atk : envelope * (1 - rel);
					outL[i] = inL[i] * envelope;
					outR[i] = inR[i] * envelope;
				}
			};
		}
		if (gate) {
			lowpass.connect(gate);
			gate.connect(ctx.destination);
		} else {
			lowpass.connect(ctx.destination);
		}
		return { lowpass, gate };
	};

	const ensureNoiseChain = async (ctx) => {
		if (noiseChain?.ctx === ctx) return noiseChain;
		if (noiseChainPromise) {
			await noiseChainPromise;
			if (noiseChain?.ctx === ctx) return noiseChain;
		}
		noiseChainPromise = buildNoiseChain(ctx).then((chain) => {
			noiseChain = chain ? { ctx, ...chain } : null;
			return noiseChain;
		});
		try {
			return await noiseChainPromise;
		} finally {
			noiseChainPromise = null;
		}
	};

	const applyNoiseParams = (chain, strength) => {
		if (!chain) return;
		const t = Math.min(1, Math.max(0, strength / 200));
		chain.lowpass.frequency.value = 11000 - t * 7000;
		const threshold = 0.02 * Math.pow(0.1, t);
		if (chain.gate?.port) chain.gate.port.postMessage({ type: 'threshold', value: threshold });
		else if (chain.gate) chain.gate.threshold = threshold;
	};

	const applyNoiseFilter = async () => {
		const speaker = emulator?.speaker_adapter;
		const mixer = speaker?.mixer;
		const ctx = speaker?.audio_context;
		if (!mixer?.node_merger || !ctx || ctx.state === 'closed') return;
		if (noiseReductionStrength <= 0) {
			if (noiseChain) {
				mixer.node_merger.disconnect();
				mixer.node_merger.connect(ctx.destination);
			}
			return;
		}
		const chain = await ensureNoiseChain(ctx);
		if (!chain) return;
		applyNoiseParams(chain, noiseReductionStrength);
		mixer.node_merger.disconnect();
		mixer.node_merger.connect(chain.lowpass);
	};

	const handleCapturedMouseMove = (event) => {
		if (!emulator || document.pointerLockElement === null) return;
		if (typeof event.movementX !== 'number' || typeof event.movementY !== 'number') return;

		// v86's bundled adapter negates movementY. Send the browser's natural
		// direction here and keep fractional 10% movement between events.
		mouseRemainderX += event.movementX * mouseSensitivity;
		mouseRemainderY += event.movementY * mouseSensitivity;
		const deltaX = mouseRemainderX < 0 ? Math.ceil(mouseRemainderX) : Math.floor(mouseRemainderX);
		const deltaY = mouseRemainderY < 0 ? Math.ceil(mouseRemainderY) : Math.floor(mouseRemainderY);
		mouseRemainderX -= deltaX;
		mouseRemainderY -= deltaY;
		if (!deltaX && !deltaY) {
			event.stopImmediatePropagation();
			return;
		}
		emulator.bus?.send?.('mouse-delta', [deltaX, deltaY]);
		event.stopImmediatePropagation();
	};

	onMount(() => {
		disposed = false;
		const keydown = (event) => {
			pressed.add(event.code);
			if (pressed.has('F8') && pressed.has('F9') && !fullscreenComboLatched) {
				fullscreenComboLatched = true;
				event.preventDefault();
				toggleFullscreen().catch(() => {});
			}
		};
		const keyup = (event) => {
			pressed.delete(event.code);
			if (!pressed.has('F8') || !pressed.has('F9')) fullscreenComboLatched = false;
		};
		const preventWheel = (event) => {
			if (!disableMouseWheel || !shell?.contains(event.target)) return;
			event.preventDefault();
			event.stopImmediatePropagation();
		};
		window.addEventListener('keydown', keydown, true);
		window.addEventListener('keyup', keyup, true);
		window.addEventListener('mousemove', handleCapturedMouseMove, true);
		window.addEventListener('wheel', preventWheel, { passive: false, capture: true });
		start();
		return () => {
			disposed = true;
			window.removeEventListener('keydown', keydown, true);
			window.removeEventListener('keyup', keyup, true);
			window.removeEventListener('mousemove', handleCapturedMouseMove, true);
			window.removeEventListener('wheel', preventWheel, { capture: true });
			destroy();
		};
	});
</script>

<Portal target={beforeDemoPortal} class="text-dark/75 text-base w-full flex flex-col">
	<span>Status: {status}</span>
	<span>Click the game to capture the mouse.</span>
	<span>
		Press <kbd>Esc</kbd>
		to release it.
	</span>
	<span>
		Press <kbd>F8</kbd>
		+
		<kbd>F9</kbd>
		together to toggle fullscreen.
	</span>
</Portal>

<Portal target={afterDemoPortal}>
	<div>
		<label class="flex items-center gap-2 mt-1 text-dark/75">
			<input type="checkbox" bind:checked={disableMouseWheel} class="accent-primary h-4 w-4" />
			<span
				class="underline decoration-dashed underline-offset-2 cursor-help"
				title="Wheel could break the some games."
			>
				Disable mousewheel
			</span>
		</label>
		<label class="flex items-center gap-2 mt-1 text-dark/75">
			<span>Noise filter:</span>
			<input
				type="range"
				min="0"
				max="200"
				step="10"
				bind:value={noiseReductionStrength}
				oninput={() => applyNoiseFilter().catch(() => {})}
				class="w-24"
			/>
			<span class="tabular-nums w-14">
				{noiseReductionStrength <= 0
					? 'None'
					: noiseReductionStrength <= 80
						? 'Light'
						: noiseReductionStrength <= 140
							? 'Medium'
							: 'Strong'}
			</span>
		</label>
		<label class="flex items-center gap-2 mt-1 text-dark/75">
			<span>Mouse sensitivity:</span>
			<input
				type="range"
				min="0.05"
				max="1"
				step="0.05"
				bind:value={mouseSensitivity}
				class="w-24"
			/>
			<span class="tabular-nums w-8">{mouseSensitivity.toFixed(2)}</span>
		</label>
	</div>
</Portal>

<div class="v86-shell absolute inset-0 grid bg-black" bind:this={shell}>
	<div
		class="screen min-h-0 min-w-0"
		bind:this={screenContainer}
		aria-label={`${title} emulator`}
		role="application"
		tabindex="0"
		onclick={() => running && captureMouse()}
		onkeydown={(event) => event.key === 'Enter' && running && captureMouse()}
	>
		<div class="font-mono text-white whitespace-pre"></div>
		<canvas class="hidden"></canvas>
	</div>
	{#if error}
		<div class="absolute inset-0 grid place-items-center bg-background p-6 text-center">
			<div class="space-y-3">
				<p class="text-accent-red font-semibold">{error}</p>
				<button class="duo-btn px-3 py-1" onclick={start}>Try again</button>
			</div>
		</div>
	{/if}
</div>

<style lang="postcss">
	@reference "../../../app.css";

	kbd {
		@apply font-bold;
	}
	.v86-shell {
		grid-template-rows: 1fr;
	}
	.screen {
		display: grid;
		place-items: center;
		overflow: hidden;
	}
	.screen :global(canvas) {
		display: block !important;
		max-width: 100%;
		max-height: 100%;
		width: auto;
		height: auto;
		image-rendering: auto !important;
	}
	.v86-shell:fullscreen {
		width: 100vw;
		height: 100vh;
		max-width: 100vw;
		max-height: 100vh;
		place-items: center;
		background: #000;
	}
	.v86-shell:fullscreen .screen {
		width: 100vw;
		height: 100vh;
	}
	.v86-shell:fullscreen :global(canvas) {
		@apply h-full! max-h-dvh w-full! max-w-dvw object-contain;
		image-rendering: auto;
	}
	input[type='range'] {
		-webkit-appearance: none;
		appearance: none;
		background: transparent;
		cursor: pointer;
		height: 1.25rem;
	}
	input[type='range']::-webkit-slider-runnable-track {
		height: 0.375rem;
		border-radius: 999px;
		background: var(--color-primary);
	}
	input[type='range']::-moz-range-track {
		height: 0.375rem;
		border-radius: 999px;
		background: var(--color-primary);
	}
	input[type='range']::-webkit-slider-thumb {
		-webkit-appearance: none;
		appearance: none;
		width: 0.875rem;
		height: 0.875rem;
		border-radius: 999px;
		background: var(--color-background);
		border: 2px solid var(--color-primary);
		margin-top: -0.25rem;
	}
	input[type='range']::-moz-range-thumb {
		width: 0.875rem;
		height: 0.875rem;
		border-radius: 999px;
		background: var(--color-background);
		border: 2px solid var(--color-primary);
	}
</style>
