import { loadSave, saveGame, clearSave, SAVE_BYTES, loadBlankFloppy } from './v86-saves.js';
import { auth } from '$lib/auth/user.svelte.js';
import { installWheelGuard, wheelBelongsToEmulator } from './wheel-guard.js';

// Weak devices (few cores) can't spare the AudioWorklet's per-sample noise gate
// on top of emulation, so default it off there: sound still plays, just without
// the DSP load. The auto frame-rate cap handles the CPU-bound case regardless.
const isWeakDevice = () => {
	const cores = globalThis.navigator?.hardwareConcurrency;
	return typeof cores === 'number' && cores > 0 && cores <= 4;
};

/** Which set of emulated devices this player builds. A restored state assumes
 *  the layout it was captured on, so any change to the device list must bump
 *  this and V86_TOPOLOGY_VERSION in v86.rs, retiring older snapshots.
 *  1 = base + game disk, cdrom, floppy, ne2k NIC
 *  2 = the same without the NIC */
export const V86_TOPOLOGY_VERSION = 2;

export class V86Player {
	status = $state('Loading emulator…');
	error = $state('');
	running = $state(false);
	mouseSensitivity = $state(0.4);
	// Per-project mouse settings from the v86 manifest, applied on top of the
	// visitor's own sensitivity slider.
	revertMouseY = $state(false);
	mouseSpeed = $state(1.0);
	noiseReductionStrength = $state(isWeakDevice() ? 0 : 200);
	disableMouseWheel = $state(true);
	audioEnabled = $state(true);
	downloadProgress = $state(null);
	screenContainer = $state();
	shell = $state();
	saveBusy = $state(false);
	saveMessage = $state('');
	paused = $state(false);
	// True once a disk read has been outstanding longer than a cache hit ever
	// takes, i.e. the game is actually blocked on a chunk fetch over the
	// network. Lets the UI explain lag instead of looking like it's frozen.
	diskFetching = $state(false);
	// Emulated instructions per second. A healthy desktop sustains tens of
	// millions; a struggling machine drops an order of magnitude, which
	// distinguishes "this device is too slow" from "this is waiting on I/O".
	emulatedMips = $state(0);

	#emulator;
	#pressed = new Set();
	#fullscreenComboLatched = false;
	#disposed = false;
	#mouseRemainderX = 0;
	#mouseRemainderY = 0;
	#noiseChain = null;
	#noiseChainPromise = null;
	#outputGain = null;
	#wheelCooldown = 0;
	#saveFloppy = null;
	#lastSaveAt = 0;
	#launcherBuffers = new Map();
	#snapshotBuffer = null;
	#snapshotHasCdrom = false;
	#carriedFloppy = null;
	#pendingReads = 0;
	#diskFetchTimer = null;
	#mipsTimer = null;
	#lastInstructionCount = 0;
	#lastMipsAt = 0;
	#progressFrame = null;
	#pendingProgress = null;
	// True once v86's emulator-ready has fired. A download-progress callback
	// coalesced into a later animation frame can otherwise run after the machine
	// is already running and rewrite the status back to "Booting…".
	#emulatorReady = false;

	saveAvailable = $derived(this.runtime?.save_supported === true);

	variants = $derived(
		Array.isArray(this.runtime?.variants) && this.runtime.variants.length > 0
			? this.runtime.variants
			: [
					{
						index: 1,
						name: '',
						iso_url: this.runtime?.iso_url,
						iso_size_bytes: this.runtime?.iso_size_bytes,
						iso_sha256: this.runtime?.iso_sha256
					}
				]
	);
	selectedVariant = $state(1);

	selected = $derived(
		this.variants.find((variant) => variant.index === this.selectedVariant) ?? this.variants[0]
	);

	// `capture` boots base + game disk only: no CD, no floppy. `snapshotHasCdrom`
	// says the in-memory buffer was captured with the disc already mounted, so
	// the studio can test-boot it with matching topology.
	constructor({
		runtime,
		mode = 'play',
		snapshotBuffer = null,
		snapshotHasCdrom = false,
		captureVariantIndex = 0
	}) {
		this.runtime = runtime;
		this.mode = mode;
		this.#snapshotBuffer = snapshotBuffer;
		this.#snapshotHasCdrom = snapshotHasCdrom;
		// 0 captures the project-wide machine with an empty drive; a variant
		// index captures with that variant's disc in, so the launcher is already
		// running and the state can be restored straight into it.
		this.captureVariantIndex = captureVariantIndex;
		if (captureVariantIndex > 0) this.selectedVariant = captureVariantIndex;
		this.revertMouseY = Boolean(runtime?.revert_mouse_y);
		this.mouseSpeed =
			typeof runtime?.mouse_speed === 'number' && runtime.mouse_speed > 0
				? runtime.mouse_speed
				: 1.0;
	}

	// variant snapshot -> project-wide snapshot -> cold boot. A variant's own
	// snapshot was captured with its disc mounted; the project-wide one was
	// captured with an empty drive. That difference decides the topology below,
	// so the two can never be swapped.
	get #snapshot() {
		if (this.mode === 'capture') return null;
		if (this.#snapshotBuffer) {
			return { buffer: this.#snapshotBuffer, hasCdrom: this.#snapshotHasCdrom };
		}
		const variant = this.selected;
		if (variant?.snapshot_url) {
			return {
				url: variant.snapshot_url,
				size: variant.snapshot_size_bytes,
				hasCdrom: true
			};
		}
		if (this.runtime?.snapshot_url) {
			return {
				url: this.runtime.snapshot_url,
				size: this.runtime.snapshot_size_bytes,
				hasCdrom: false
			};
		}
		return null;
	}

	get usingSnapshot() {
		return this.#snapshot !== null;
	}

	// Only the empty-drive snapshot defers the disc. A snapshot captured with
	// the disc in must get it back at construction: v86 throws if the restored
	// state and the attached drives disagree.
	get #deferMedia() {
		const snapshot = this.#snapshot;
		return Boolean(snapshot) && !snapshot.hasCdrom;
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

	#buildOptions = () => {
		const options = {
			wasm_path: '/v86/build/v86.wasm',
			screen: {
				container: this.screenContainer,
				use_graphical_text: false
			},
			screen_container: this.screenContainer,
			autostart: true,
			memory_size: this.runtime.memory_size,
			vga_memory_size: this.runtime.vga_memory_size,
			boot_order: 786,
			bios: { url: '/v86/bios/seabios.bin' },
			vga_bios: { url: '/v86/bios/vgabios.bin' },
			hda: {
				url: this.runtime.base_url,
				size: this.runtime.base_size_bytes,
				async: true,
				fixed_chunk_size: this.runtime.chunk_size_bytes,
				use_parts: true
			},
			hdb: {
				url: this.runtime.game_url,
				size: this.runtime.game_size_bytes,
				async: true,
				fixed_chunk_size: this.runtime.chunk_size_bytes,
				use_parts: true
			},
			acpi: false,
			disable_speaker: false
			// No net_device: it was an ne2k with no relay, so the guest spent time
			// initialising and polling hardware that could never carry traffic.
		};

		// Disks must be present at construction: Windows enumerates IDE disks
		// at boot and never hot-detects one.
		const snapshot = this.#snapshot;
		if (snapshot) {
			options.initial_state = snapshot.buffer
				? { buffer: snapshot.buffer }
				: { url: snapshot.url, size: snapshot.size };
		}

		const cdrom = {
			url: this.selected?.iso_url ?? this.runtime.iso_url,
			size: this.selected?.iso_size_bytes ?? this.runtime.iso_size_bytes,
			async: false
		};

		// Capture never mounts a floppy, whichever kind it is: the state has to
		// restore into an empty A: so the launcher copies the visitor's own save.
		if (this.mode === 'capture') {
			if (this.captureVariantIndex > 0) options.cdrom = cdrom;
			return options;
		}

		if (!this.#deferMedia) {
			options.cdrom = cdrom;
			// Same reason in reverse: under any snapshot the floppy is attached
			// after the restore, never baked into the machine at construction.
			if (!snapshot && this.#saveFloppy) options.fda = { buffer: this.#saveFloppy.buffer };
		}
		return options;
	};

	// set_cdrom/insert_disk raise media_changed, which is what triggers autorun.
	attachDeferredMedia = async () => {
		if (!this.#emulator || this.mode === 'capture') return;
		// Floppy first: inserting the CD is what fires autorun, and the launcher
		// copies saves off A: early, so A: has to already be there. Under a
		// variant snapshot the launcher is already running and waiting out its
		// delay, which is what gives the floppy time to land.
		if (this.#saveFloppy) {
			await this.#emulator.set_fda?.({ buffer: this.#saveFloppy.buffer });
		}
		if (!this.#deferMedia) return;
		const iso = this.selected?.iso_url ?? this.runtime.iso_url;
		if (iso) {
			const buffer = this.#launcherBuffers.get(this.selectedVariant);
			await this.#emulator.set_cdrom?.(buffer ? { buffer } : { url: iso, async: false });
		}
	};

	// ide-read-start/end bracket every disk read, cached or not. A cache hit
	// resolves within the same tick; only a real chunk fetch takes long enough
	// to clear this threshold, so this stays quiet during normal play.
	#onDiskReadStart = () => {
		this.#pendingReads++;
		if (this.#diskFetchTimer) return;
		this.#diskFetchTimer = setTimeout(() => {
			this.#diskFetchTimer = null;
			if (this.#pendingReads > 0) this.diskFetching = true;
		}, 150);
	};

	#onDiskReadEnd = () => {
		this.#pendingReads = Math.max(0, this.#pendingReads - 1);
		if (this.#pendingReads > 0) return;
		if (this.#diskFetchTimer) {
			clearTimeout(this.#diskFetchTimer);
			this.#diskFetchTimer = null;
		}
		this.diskFetching = false;
	};

	// The counter is a uint32 that wraps, so only deltas are meaningful and a
	// wrap is discarded rather than reported as a spike.
	#startMipsSampling = () => {
		this.#stopMipsSampling();
		this.#lastInstructionCount = this.#emulator?.get_instruction_counter?.() ?? 0;
		this.#lastMipsAt = performance.now();
		this.#mipsTimer = setInterval(() => {
			const count = this.#emulator?.get_instruction_counter?.();
			if (count == null) return;
			const now = performance.now();
			const elapsed = (now - this.#lastMipsAt) / 1000;
			const delta = count - this.#lastInstructionCount;
			if (elapsed > 0 && delta >= 0) {
				this.emulatedMips = delta / elapsed / 1e6;
			}
			this.#lastInstructionCount = count;
			this.#lastMipsAt = now;
		}, 1000);
	};

	#stopMipsSampling = () => {
		if (this.#mipsTimer) clearInterval(this.#mipsTimer);
		this.#mipsTimer = null;
	};

	start = async () => {
		if (this.running || !this.runtime) return;
		this.error = '';
		this.status = 'Loading emulator…';
		this.saveMessage = '';
		try {
			if ('serviceWorker' in navigator) {
				await navigator.serviceWorker.register('/v86-cache-worker.js', { scope: '/' });
				await navigator.serviceWorker.ready;
			}
			await this.loadRuntime();
			if (this.#disposed) return;
			const V86Constructor = window.V86 ?? window.V86Starter;
			if (!V86Constructor) throw new Error('The local v86 constructor is unavailable.');
			if (this.mode !== 'capture' && this.runtime.save_supported) {
				if (this.#carriedFloppy) {
					this.#saveFloppy = this.#carriedFloppy;
					this.#carriedFloppy = null;
				} else {
					this.status = 'Loading your save…';
					this.#saveFloppy =
						(await loadSave(this.runtime.slug).catch(() => null)) ??
						(await loadBlankFloppy().catch(() => null));
					if (this.#disposed) return;
				}
			}
			this.#emulator = new V86Constructor(this.#buildOptions());
			installWheelGuard(this.#emulator, () => this.disableMouseWheel);
			if (this.#disposed) {
				await this.destroy();
				return;
			}
			this.running = true;
			this.#emulatorReady = false;
			this.status = this.usingSnapshot
				? `Restoring ${this.runtime.system_name}…`
				: `Booting ${this.runtime.system_name}…`;
			this.#emulator.add_listener?.('emulator-ready', () => {
				this.#emulatorReady = true;
				this.status = 'Running';
				this.applyNoiseFilter().catch(() => {});
				this.applyAudioState();
				this.preloadLaunchers().catch(() => {});
				this.#startMipsSampling();
			});
			// emulator-ready fires inside v86.init(), before restore_state runs.
			// Inserting media there leaves a disc in a drive the state records as
			// empty, and the restore then crashes. emulator-loaded is post-restore.
			this.#emulator.add_listener?.('emulator-loaded', () => {
				if (!this.usingSnapshot) return;
				this.status = this.#deferMedia ? 'Inserting disc…' : 'Loading your save…';
				this.attachDeferredMedia()
					.then(() => {
						this.status = 'Running';
					})
					.catch((cause) => {
						this.error = cause?.message ?? 'Could not insert the disc.';
					});
			});
			this.#emulator.add_listener?.('ide-read-start', this.#onDiskReadStart);
			this.#emulator.add_listener?.('ide-read-end', this.#onDiskReadEnd);
			this.#emulator.add_listener?.('download-progress', (info) => {
				const fileUrl = new URL(info?.file_name ?? '', window.location.origin);
				const gameAsset = fileUrl.pathname.includes(`/${this.runtime.game_sha256}/`);
				const isoPaths = this.variants.map(
					(variant) => new URL(variant.iso_url, window.location.origin).pathname
				);
				const isIso =
					isoPaths.includes(fileUrl.pathname) ||
					fileUrl.pathname === new URL(this.runtime.iso_url, window.location.origin).pathname;
				if (!gameAsset && !isIso) return;
				if (info.loaded == null || info.total == null) return;
				// Chunks arrive in bursts; coalesce to one render per frame so
				// the burst doesn't churn the DOM during boot.
				this.#pendingProgress = { loaded: info.loaded, total: info.total };
				if (this.#progressFrame) return;
				this.#progressFrame = requestAnimationFrame(() => {
					this.#progressFrame = null;
					const { loaded, total } = this.#pendingProgress ?? {};
					this.#pendingProgress = null;
					if (loaded == null || total == null) return;
					this.downloadProgress = Math.min(100, (loaded / total) * 100);
					if (loaded >= total) {
						this.downloadProgress = null;
						// The machine may already be running by the time this frame
						// lands; don't drag the status back to "Booting…".
						if (!this.#emulatorReady) {
							this.status = `Booting ${this.runtime.system_name}…`;
						}
					} else if (!this.#emulatorReady) {
						const mLoaded = Math.floor(loaded / 1048576);
						const mTotal = Math.floor(total / 1048576);
						this.status = `Downloading game (${mLoaded} / ${mTotal} MB)`;
					}
				});
			});
			this.applyNoiseFilter().catch(() => {});
		} catch (cause) {
			this.error = cause?.message ?? 'v86 failed to start.';
			this.status = '';
			this.running = false;
		}
	};

	// stop() resolves once the CPU loop has halted, so a state captured after
	// awaiting it is consistent.
	pause = async () => {
		if (!this.#emulator || this.paused) return;
		await this.#emulator.stop?.();
		this.paused = true;
		this.status = 'Paused';
	};

	resume = async () => {
		if (!this.#emulator || !this.paused) return;
		await this.#emulator.run?.();
		this.paused = false;
		this.status = 'Running';
	};

	// Returns the raw, uncompressed state.
	captureState = async () => {
		if (!this.#emulator) throw new Error('The emulator is not running.');
		await this.pause();
		const state = await this.#emulator.save_state?.();
		if (!state) throw new Error('v86 did not return a state.');
		return state;
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

	// v86's destroy() is async and unbinds adapters after awaiting stop(), so
	// anything rebuilding the machine must await this first.
	destroy = async () => {
		const emulator = this.#emulator;
		// Cleared up front so a concurrent start() can never see the dying one.
		this.#emulator = undefined;
		this.running = false;
		this.paused = false;
		this.#emulatorReady = false;
		this.#pendingReads = 0;
		this.diskFetching = false;
		this.emulatedMips = 0;
		this.#stopMipsSampling();
		if (this.#progressFrame) {
			cancelAnimationFrame(this.#progressFrame);
			this.#progressFrame = null;
		}
		this.#pendingProgress = null;
		if (this.#diskFetchTimer) {
			clearTimeout(this.#diskFetchTimer);
			this.#diskFetchTimer = null;
		}
		try {
			if (emulator?.destroy) await emulator.destroy();
			else await emulator?.stop?.();
		} catch {
			// A machine that died mid-boot may not tear down cleanly.
		} finally {
			for (const context of document.querySelectorAll('audio')) {
				if (this.shell?.contains(context)) context.remove();
			}
		}
	};

	restart = async () => {
		if (!this.runtime || this.#disposed) return;
		await this.destroy();
		this.downloadProgress = null;
		await this.start();
	};

	// Rebuilds around a state blob so the studio can dry-run a capture.
	// Pass null to go back to a cold boot.
	rebootWithSnapshot = async (buffer, { mode = 'play', hasCdrom = false } = {}) => {
		if (!this.runtime || this.#disposed) return;
		await this.destroy();
		this.#snapshotBuffer = buffer;
		this.#snapshotHasCdrom = hasCdrom;
		this.mode = mode;
		this.downloadProgress = null;
		this.error = '';
		await this.start();
	};

	// Preloads each variant's launcher CD so the post-restore insert is instant.
	preloadLaunchers = async () => {
		const jobs = this.variants.map(async (variant) => {
			if (this.#launcherBuffers.has(variant.index)) return;
			try {
				const res = await fetch(variant.iso_url);
				if (!res.ok) return;
				this.#launcherBuffers.set(variant.index, await res.arrayBuffer());
			} catch {
				// attachDeferredMedia falls back to inserting by URL
			}
		});
		await Promise.allSettled(jobs);
	};

	// Full rebuild, same path as first load. v86's own restart() only resets the
	// CPU, leaving the disks' write cache half-written, which corrupts the guest.
	selectVariant = async (index) => {
		const variant = this.variants.find((v) => v.index === index);
		if (!variant || variant.index === this.selectedVariant) return;
		this.selectedVariant = index;
		try {
			if (this.#emulator && this.running && this.runtime?.save_supported) {
				this.status = 'Saving game…';
				const floppy = this.#emulator.get_disk_fda?.();
				if (
					floppy &&
					floppy.length === SAVE_BYTES &&
					floppy[510] === 0x55 &&
					floppy[511] === 0xaa
				) {
					this.#carriedFloppy = floppy.slice();
					await saveGame(this.runtime.slug, floppy).catch(() => {});
				}
			}
			this.status = `Loading ${variant.name || 'variant'}…`;
			await this.restart();
		} catch (cause) {
			this.error = cause?.message ?? 'Could not switch variant.';
		}
	};

	saveNow = async () => {
		if (!this.runtime?.save_supported || !this.running) return;
		const now = Date.now();
		if (now - this.#lastSaveAt < 30000) {
			this.saveMessage = 'Please wait 30 seconds between saves.';
			return;
		}
		const floppy = this.#emulator?.get_disk_fda?.();
		if (!floppy || floppy.length !== SAVE_BYTES || floppy[510] !== 0x55 || floppy[511] !== 0xaa) {
			this.saveMessage = 'The save floppy is not ready yet.';
			return;
		}
		this.saveBusy = true;
		this.saveMessage = 'Saving…';
		try {
			await saveGame(this.runtime.slug, floppy);
			this.#lastSaveAt = now;
			this.saveMessage = auth() ? 'Game saved to your account.' : 'Game saved to this browser.';
		} catch (cause) {
			this.saveMessage = cause?.message ?? 'Could not save the game.';
		} finally {
			this.saveBusy = false;
		}
	};

	clearNow = async () => {
		if (!this.runtime?.save_supported || this.saveBusy) return;
		this.saveBusy = true;
		this.saveMessage = 'Clearing…';
		try {
			await clearSave(this.runtime.slug);
			this.#lastSaveAt = 0;
			this.saveMessage = 'Save cleared.';
		} catch (cause) {
			this.saveMessage = cause?.message ?? 'Could not clear the save.';
		} finally {
			this.saveBusy = false;
		}
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
		// The gate only ever runs as an AudioWorklet, i.e. on the audio thread.
		// There is deliberately no ScriptProcessor fallback: that variant runs
		// its per-sample callback on the main thread, competing with the
		// emulator loop, and it was reached by losing a race against a timeout —
		// so the slowest machines, which can least afford it, were the ones that
		// got it. Without a worklet the chain degrades to the native lowpass,
		// which is C++ and effectively free.
		let gate = null;
		try {
			if (ctx.audioWorklet?.addModule) {
				const url = V86Player.createNoiseGateWorkletUrl();
				await Promise.race([
					ctx.audioWorklet.addModule(url),
					new Promise((_, reject) =>
						setTimeout(() => reject(new Error('worklet load timed out')), 5000)
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
			console.warn('v86 noise-gate worklet unavailable, using lowpass only.', cause);
		}
		if (gate) {
			lowpass.connect(gate);
			gate.connect(this.ensureMasterGain(ctx));
		} else {
			lowpass.connect(this.ensureMasterGain(ctx));
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
		chain.gate?.port?.postMessage({ type: 'threshold', value: threshold });
	};

	applyNoiseFilter = async () => {
		const speaker = this.#emulator?.speaker_adapter;
		const mixer = speaker?.mixer;
		const ctx = speaker?.audio_context;
		if (!mixer?.node_merger || !ctx || ctx.state === 'closed') return;
		if (this.noiseReductionStrength <= 0) {
			if (this.#noiseChain) {
				mixer.node_merger.disconnect();
				mixer.node_merger.connect(this.ensureMasterGain(ctx));
			}
			return;
		}
		const chain = await this.ensureNoiseChain(ctx);
		if (!chain) return;
		this.applyNoiseParams(chain, this.noiseReductionStrength);
		mixer.node_merger.disconnect();
		mixer.node_merger.connect(chain.lowpass);
	};

	// One master GainNode sits in front of ctx.destination, and every path that
	// used to connect straight to destination (the noise chain's output and the
	// bypass in applyNoiseFilter) now routes through it. Muting pauses by gain,
	// never by suspend: suspending freezes the AudioContext clock, which starves
	// the emulated sound card's DMA and makes games that wait on it go erratic.
	ensureMasterGain = (ctx) => {
		if (this.#outputGain && this.#outputGain.context === ctx) return this.#outputGain;
		this.#outputGain = ctx.createGain();
		this.#outputGain.gain.value = 1;
		this.#outputGain.connect(ctx.destination);
		return this.#outputGain;
	};

	applyAudioState = () => {
		const speaker = this.#emulator?.speaker_adapter;
		const ctx = speaker?.audio_context;
		if (!ctx || ctx.state === 'closed') return;
		const gain = this.ensureMasterGain(ctx);
		gain.gain.value = this.audioEnabled ? 1 : 0;
		// A user gesture is required before the UA will start an AudioContext.
		// The mute toggle only flips gain, so the clock keeps running and the
		// pipeline keeps draining; resume stays solely for that initial unlock.
		if (ctx.state === 'suspended') ctx.resume().catch(() => {});
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

		// v86's bundled adapter negates movementY, which is the direction the
		// guest expects. `revert_mouse_y` (from the project manifest) sends the
		// browser's natural direction instead, and `mouse_speed` scales the
		// project's preferred mouse speed on top of the visitor's slider.
		const ySign = this.revertMouseY ? 1 : -1;
		this.#mouseRemainderX += event.movementX * this.mouseSensitivity * this.mouseSpeed;
		this.#mouseRemainderY += event.movementY * ySign * this.mouseSensitivity * this.mouseSpeed;
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
		if (!wheelBelongsToEmulator(event, this.shell)) return;
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
		// Svelte's cleanup is synchronous; #disposed already blocks any restart,
		// so letting the teardown finish in the background is safe here.
		this.destroy();
	}
}
