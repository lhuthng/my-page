export class V86Player {
	status = $state('Loading emulator…');
	error = $state('');
	running = $state(false);
	mouseSensitivity = $state(0.4);
	noiseReductionStrength = $state(200);
	disableMouseWheel = $state(true);
	audioEnabled = $state(true);
	downloadProgress = $state(null);
	screenContainer = $state();
	shell = $state();

	#emulator;
	#pressed = new Set();
	#fullscreenComboLatched = false;
	#disposed = false;
	#mouseRemainderX = 0;
	#mouseRemainderY = 0;
	#noiseChain = null;
	#noiseChainPromise = null;
	#wheelCooldown = 0;

	constructor({ runtime }) {
		this.runtime = runtime;
	}

	loadRuntime = () =>
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

	start = async () => {
		if (this.running || !this.runtime) return;
		this.error = '';
		this.status = 'Loading emulator…';
		try {
			if ('serviceWorker' in navigator) {
				await navigator.serviceWorker.register('/v86-cache-worker.js', { scope: '/' });
				await navigator.serviceWorker.ready;
			}
			await this.loadRuntime();
			if (this.#disposed) return;
			const V86Constructor = window.V86 ?? window.V86Starter;
			if (!V86Constructor) throw new Error('The local v86 constructor is unavailable.');
			this.#emulator = new V86Constructor({
				wasm_path: '/v86/build/v86.wasm',
				screen: {
					container: this.screenContainer,
					use_graphical_text: false
				},
				screen_container: this.screenContainer,
				autostart: true,
				memory_size: this.runtime.memory_size,
				vga_memory_size: this.runtime.vga_memory_size,
				boot_order: 0,
				bios: { url: '/v86/bios/seabios.bin' },
				vga_bios: { url: '/v86/bios/vgabios.bin' },
				hda: {
					url: this.runtime.base_url,
					size: this.runtime.base_size_bytes,
					async: true,
					fixed_chunk_size: this.runtime.chunk_size_bytes,
					use_parts: true
				},
				cdrom: {
					url: this.runtime.iso_url,
					size: this.runtime.iso_size_bytes,
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
			if (this.#disposed) {
				this.destroy();
				return;
			}
			this.running = true;
			this.status = `Booting ${this.runtime.system_name}…`;
			this.#emulator.add_listener?.('emulator-ready', () => {
				this.status = 'Running';
				this.applyNoiseFilter().catch(() => {});
				this.applyAudioState();
			});
			this.#emulator.add_listener?.('download-progress', (info) => {
				if (info?.file_name !== this.runtime.iso_url) return;
				if (info.loaded == null || info.total == null) return;
				this.downloadProgress = Math.min(100, (info.loaded / info.total) * 100);
				if (info.loaded >= info.total) {
					this.downloadProgress = null;
					this.status = `Booting ${this.runtime.system_name}…`;
				} else {
					const loaded = Math.floor(info.loaded / 1048576);
					const total = Math.floor(info.total / 1048576);
					this.status = `Downloading ISO (${loaded} / ${total} MB)`;
				}
			});
			this.applyNoiseFilter().catch(() => {});
		} catch (cause) {
			this.error = cause?.message ?? 'v86 failed to start.';
			this.status = '';
			this.running = false;
		}
	};

	captureMouse = () => {
		if (typeof this.#emulator?.lock_mouse === 'function') this.#emulator.lock_mouse();
		else this.screenContainer?.querySelector('canvas')?.requestPointerLock?.();
	};

	toggleFullscreen = async () => {
		if (!document.fullscreenElement) {
			await this.shell?.requestFullscreen?.();
		} else {
			await document.exitFullscreen?.();
		}
	};

	destroy = () => {
		try {
			this.#emulator?.stop?.();
			this.#emulator?.destroy?.();
		} finally {
			this.#emulator = undefined;
			this.running = false;
			for (const context of document.querySelectorAll('audio')) {
				if (this.shell?.contains(context)) context.remove();
			}
		}
	};

	restart = async () => {
		if (!this.runtime || this.#disposed) return;
		this.destroy();
		this.downloadProgress = null;
		await this.start();
	};

	static createNoiseGateWorkletUrl() {
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
	}

	buildNoiseChain = async (ctx) => {
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
				const url = V86Player.createNoiseGateWorkletUrl();
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

	ensureNoiseChain = async (ctx) => {
		if (this.#noiseChain?.ctx === ctx) return this.#noiseChain;
		if (this.#noiseChainPromise) {
			await this.#noiseChainPromise;
			if (this.#noiseChain?.ctx === ctx) return this.#noiseChain;
		}
		this.#noiseChainPromise = this.buildNoiseChain(ctx).then((chain) => {
			this.#noiseChain = chain ? { ctx, ...chain } : null;
			return this.#noiseChain;
		});
		try {
			return await this.#noiseChainPromise;
		} finally {
			this.#noiseChainPromise = null;
		}
	};

	applyNoiseParams = (chain, strength) => {
		if (!chain) return;
		const t = Math.min(1, Math.max(0, strength / 200));
		chain.lowpass.frequency.value = 11000 - t * 7000;
		const threshold = 0.02 * Math.pow(0.1, t);
		if (chain.gate?.port) chain.gate.port.postMessage({ type: 'threshold', value: threshold });
		else if (chain.gate) chain.gate.threshold = threshold;
	};

	applyNoiseFilter = async () => {
		const speaker = this.#emulator?.speaker_adapter;
		const mixer = speaker?.mixer;
		const ctx = speaker?.audio_context;
		if (!mixer?.node_merger || !ctx || ctx.state === 'closed') return;
		if (this.noiseReductionStrength <= 0) {
			if (this.#noiseChain) {
				mixer.node_merger.disconnect();
				mixer.node_merger.connect(ctx.destination);
			}
			return;
		}
		const chain = await this.ensureNoiseChain(ctx);
		if (!chain) return;
		this.applyNoiseParams(chain, this.noiseReductionStrength);
		mixer.node_merger.disconnect();
		mixer.node_merger.connect(chain.lowpass);
	};

	applyAudioState = () => {
		const ctx = this.#emulator?.speaker_adapter?.audio_context;
		if (!ctx || ctx.state === 'closed') return;
		if (this.audioEnabled) {
			if (ctx.state === 'suspended') ctx.resume().catch(() => {});
		} else if (ctx.state === 'running') {
			ctx.suspend().catch(() => {});
		}
	};

	handleMiddleClick = (event) => {
		if (event.which !== 2 && event.button !== 1) return;
		if (document.pointerLockElement === null && !this.shell?.contains(event.target)) return;
		event.preventDefault();
		event.stopImmediatePropagation();
	};

	handleCapturedMouseMove = (event) => {
		if (!this.#emulator || document.pointerLockElement === null) return;
		if (typeof event.movementX !== 'number' || typeof event.movementY !== 'number') return;

		// v86's bundled adapter negates movementY. Send the browser's natural
		// direction here and keep fractional 10% movement between events.
		this.#mouseRemainderX += event.movementX * this.mouseSensitivity;
		this.#mouseRemainderY += event.movementY * this.mouseSensitivity;
		const deltaX =
			this.#mouseRemainderX < 0
				? Math.ceil(this.#mouseRemainderX)
				: Math.floor(this.#mouseRemainderX);
		const deltaY =
			this.#mouseRemainderY < 0
				? Math.ceil(this.#mouseRemainderY)
				: Math.floor(this.#mouseRemainderY);
		this.#mouseRemainderX -= deltaX;
		this.#mouseRemainderY -= deltaY;
		if (!deltaX && !deltaY) {
			event.stopImmediatePropagation();
			return;
		}
		this.#emulator.bus?.send?.('mouse-delta', [deltaX, deltaY]);
		event.stopImmediatePropagation();
	};

	handleKeyDown = (event) => {
		this.#pressed.add(event.code);
		if (this.#pressed.has('F8') && this.#pressed.has('F9') && !this.#fullscreenComboLatched) {
			this.#fullscreenComboLatched = true;
			event.preventDefault();
			this.toggleFullscreen().catch(() => {});
		}
	};

	handleKeyUp = (event) => {
		this.#pressed.delete(event.code);
		if (!this.#pressed.has('F8') || !this.#pressed.has('F9')) this.#fullscreenComboLatched = false;
	};

	handleWheel = (event) => {
		if (!this.shell?.contains(event.target)) return;
		event.preventDefault();
		event.stopImmediatePropagation();
		if (this.disableMouseWheel) return;
		const delta = event.deltaY !== 0 ? event.deltaY : event.deltaX;
		const direction = delta > 0 ? 1 : delta < 0 ? -1 : 0;
		if (!direction) return;
		const now = performance.now();
		if (now - this.#wheelCooldown < 25) return;
		this.#wheelCooldown = now;
		this.#emulator?.bus?.send?.('mouse-wheel', [direction, 0]);
	};

	mount() {
		this.#disposed = false;
		window.addEventListener('keydown', this.handleKeyDown, true);
		window.addEventListener('keyup', this.handleKeyUp, true);
		window.addEventListener('mousedown', this.handleMiddleClick, true);
		window.addEventListener('mouseup', this.handleMiddleClick, true);
		window.addEventListener('mousemove', this.handleCapturedMouseMove, true);
		window.addEventListener('wheel', this.handleWheel, { passive: false, capture: true });
		this.start();
	}

	unmount() {
		this.#disposed = true;
		window.removeEventListener('keydown', this.handleKeyDown, true);
		window.removeEventListener('keyup', this.handleKeyUp, true);
		window.removeEventListener('mousedown', this.handleMiddleClick, true);
		window.removeEventListener('mouseup', this.handleMiddleClick, true);
		window.removeEventListener('mousemove', this.handleCapturedMouseMove, true);
		window.removeEventListener('wheel', this.handleWheel, { capture: true });
		this.destroy();
	}
}
