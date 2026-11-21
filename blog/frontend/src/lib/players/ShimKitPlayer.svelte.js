// Loads a web bundle's shim-kit payload and drives the kit's browser ABI.
//
// The boot contract is the kit's `docs/loading-a-payload.md`; the reference
// loader that runs Doraemon lives in
// `doraemon-monopoly-localization/tools/web-client`, and this class is its
// port into the site's player shape: a canvas, a frame loop, the audio-event
// queue, and — where the reference page had localStorage — the site's unified
// per-user save files (`lib/saves/game-saves.js`).
//
// A bundle (`docs/web-bundle-contract.md` in the kit) is `bundle.json` plus,
// per variant, `artifact.json`, `host_wasm.wasm` and `game/files.json`. A
// manifest without `variants` is one variant rooted at the manifest itself.
//
// One detail shapes the code, as it shapes every loader of this ABI: `wh_alloc`
// is a single reused scratch buffer, so a name and its bytes go into one
// allocation and anything needed later is copied out before the next call.

import { listSaves, loadSaveFile, saveSaveFile, deleteSaveFile } from '$lib/saves/game-saves.js';

/** Exports that answer with a pointer into the module's memory. */
const POINTER_EXPORTS = [
	'wh_alloc',
	'wh_memory_ptr',
	'wh_message_ptr',
	'wh_saved_ptr',
	'wh_audio_events_ptr',
	'wh_audio_pcm_ptr',
	'wh_framebuffer_offset'
];

// `KeyboardEvent.code` to PC set 1 scancodes, which is what DirectInput names.
const KEY_CODES = {
	Escape: 0x01,
	Digit1: 0x02,
	Digit2: 0x03,
	Digit3: 0x04,
	Digit4: 0x05,
	Digit5: 0x06,
	Digit6: 0x07,
	Digit7: 0x08,
	Digit8: 0x09,
	Digit9: 0x0a,
	Digit0: 0x0b,
	Minus: 0x0c,
	Equal: 0x0d,
	Backspace: 0x0e,
	Tab: 0x0f,
	KeyQ: 0x10,
	KeyW: 0x11,
	KeyE: 0x12,
	KeyR: 0x13,
	KeyT: 0x14,
	KeyY: 0x15,
	KeyU: 0x16,
	KeyI: 0x17,
	KeyO: 0x18,
	KeyP: 0x19,
	BracketLeft: 0x1a,
	BracketRight: 0x1b,
	Enter: 0x1c,
	ControlLeft: 0x1d,
	KeyA: 0x1e,
	KeyS: 0x1f,
	KeyD: 0x20,
	KeyF: 0x21,
	KeyG: 0x22,
	KeyH: 0x23,
	KeyJ: 0x24,
	KeyK: 0x25,
	KeyL: 0x26,
	Semicolon: 0x27,
	Quote: 0x28,
	Backquote: 0x29,
	ShiftLeft: 0x2a,
	Backslash: 0x2b,
	KeyZ: 0x2c,
	KeyX: 0x2d,
	KeyC: 0x2e,
	KeyV: 0x2f,
	KeyB: 0x30,
	KeyN: 0x31,
	KeyM: 0x32,
	Comma: 0x33,
	Period: 0x34,
	Slash: 0x35,
	ShiftRight: 0x36,
	NumpadMultiply: 0x37,
	AltLeft: 0x38,
	Space: 0x39,
	CapsLock: 0x3a,
	F1: 0x3b,
	F2: 0x3c,
	F3: 0x3d,
	F4: 0x3e,
	F5: 0x3f,
	F6: 0x40,
	F7: 0x41,
	F8: 0x42,
	F9: 0x43,
	F10: 0x44,
	NumLock: 0x45,
	ScrollLock: 0x46,
	Numpad7: 0x47,
	Numpad8: 0x48,
	Numpad9: 0x49,
	NumpadSubtract: 0x4a,
	Numpad4: 0x4b,
	Numpad5: 0x4c,
	Numpad6: 0x4d,
	NumpadAdd: 0x4e,
	Numpad1: 0x4f,
	Numpad2: 0x50,
	Numpad3: 0x51,
	Numpad0: 0x52,
	NumpadDecimal: 0x53,
	Insert: 0xd2,
	Delete: 0xd3,
	Home: 0xc7,
	End: 0xcf,
	PageUp: 0xc9,
	PageDown: 0xd1,
	ArrowUp: 0xc8,
	ArrowDown: 0xd0,
	ArrowLeft: 0xcb,
	ArrowRight: 0xcd
};

/** Keys the page must keep for the game rather than the browser. */
const CAPTURED = new Set([
	'ArrowUp',
	'ArrowDown',
	'ArrowLeft',
	'ArrowRight',
	'Space',
	'Tab',
	'Backspace',
	'Enter',
	'Slash',
	'Quote'
]);

const AUDIO_EVENT_PLAY = 0;
const AUDIO_EVENT_STOP = 2;
const AUDIO_EVENT_RELEASE = 3;
const AUDIO_EVENT_DATA = 4;
const AUDIO_EVENT_BYTES = 44;

/** The last segment of a path, lowercased: the guest names saves like folders list them. */
function leafName(path) {
	return path.toLowerCase().split(/[\\/]/).pop();
}

/**
 * Whether a file is one the bundle calls a save. `*` takes everything, and a
 * bundle that names nothing keeps everything the guest writes.
 */
export function keptBy(patterns, name) {
	if (!patterns || patterns.length === 0) return true;
	const leaf = leafName(name);
	return patterns.some((pattern) => pattern === '*' || leafName(pattern) === leaf);
}

/** Whether two save snapshots hold the same bytes. Saves are small; every byte. */
function sameBytes(a, b) {
	if (a === b) return true;
	if (!a || !b || a.length !== b.length) return false;
	for (let at = 0; at < a.length; at += 1) {
		if (a[at] !== b[at]) return false;
	}
	return true;
}

export class ShimKitPlayer {
	/**
	 * @param {object} args
	 * @param {string} args.slug the game's slug; save files are keyed by it
	 * @param {string} [args.payloadUrl] where the bundle is served; defaults to
	 *   the site's own API proxy for the slug
	 */
	constructor({ slug, payloadUrl, onStatus = () => {}, onDiagnostic = () => {} }) {
		this.slug = slug;
		this.base = (payloadUrl ?? `/api/games/s/${encodeURIComponent(slug)}/payload/`).endsWith('/')
			? (payloadUrl ?? `/api/games/s/${encodeURIComponent(slug)}/payload/`)
			: `${payloadUrl}/`;
		this.onStatus = onStatus;
		this.onDiagnostic = onDiagnostic;
		this.canvas = null;
		this.bundle = null;
		this.variants = [];
		this.selectedVariant = '';
		this.executables = [];
		this.selectedExe = '';
		this.state = 'idle'; // idle | loading | ready | running | paused | error
		this.errorMessage = '';
		this.booted = false;
		this.audio = null;
		this.audioEnabled = true;
		this.voices = new Map();
		this.handle = 0;
		// The guest's files as they stood at the last persist, name -> Uint8Array.
		this.persisted = new Map();
		this.savedVersion = 0;
		this.persistedAt = 0;
		this.lastMessage = '';
		this.flips = -1;
		// Writes to the presented surface that no Flip counted: a guest that
		// blits straight onto the primary never moves the flip counter, and
		// Cave Story is exactly that guest — without this version the screen
		// stays black while the guest runs happily behind it.
		this.version = -1;
		this.paletteVersion = -1;
		this.palette = new Uint32Array(256);
		this.image = null;
		this.shell = null;
	}

	/** The phases the shared player chrome colours its status dot with. */
	get phase() {
		switch (this.state) {
			case 'running':
				return 'running';
			case 'error':
				return 'error';
			case 'loading':
				return 'loading';
			case 'idle':
			case 'ready':
				return 'idle';
			default:
				return 'booting';
		}
	}

	status(text, kind = '') {
		this.statusKind = kind;
		this.onStatus(text, kind);
	}

	/** The page's audio context, created from the first user gesture. */
	audioContext() {
		if (!this.audio) {
			const Constructor = window.AudioContext ?? window.webkitAudioContext;
			if (!Constructor) return null;
			this.audio = new Constructor();
		}
		if (this.audio.state === 'suspended') this.audio.resume().catch(() => {});
		return this.audio;
	}

	/**
	 * Creates the context on the first gesture of the session, so the title
	 * screen's music is audible from the first click, and applies the sound
	 * toggle the chrome's popover binds (the v86 player's `applyAudioState`).
	 */
	ensureAudioOnGesture() {
		const gesture = () => {
			this.audioContext();
			this.applyAudioState();
			window.removeEventListener('pointerdown', gesture);
			window.removeEventListener('keydown', gesture);
		};
		window.addEventListener('pointerdown', gesture);
		window.addEventListener('keydown', gesture);
	}

	/** Applies the sound toggle: muted suspends the context entirely. */
	applyAudioState() {
		if (!this.audio) return;
		if (this.audioEnabled && this.audio.state === 'suspended') {
			this.audio.resume().catch(() => {});
		} else if (!this.audioEnabled && this.audio.state === 'running') {
			this.audio.suspend().catch(() => {});
		}
	}

	async fetchJson(url, optional = false) {
		const response = await fetch(url, { cache: 'no-store' });
		if (!response.ok) {
			if (optional) return null;
			throw new Error(`HTTP ${response.status} for ${url}`);
		}
		return response.json();
	}

	/** Reads the bundle manifest and remembers which variants it offers. */
	async mount(canvas) {
		this.canvas = canvas;
		this.state = 'loading';
		// Console-reachable for a session that goes wrong mid-game: the player
		// owns the running host, so a frozen frame can be probed from here.
		window.__shimkitPlayer = this;
		this.ensureAudioOnGesture();
		try {
			this.bundle = await this.fetchJson(`${this.base}bundle.json`);
			const listed = Array.isArray(this.bundle?.variants) ? this.bundle.variants : [];
			this.variants = listed.map((variant) => ({ name: variant.name, dir: variant.dir }));
			this.selectedVariant = this.variants[0]?.dir ?? '';
			await this.bootVariant(this.selectedVariant);
		} catch (error) {
			this.state = 'error';
			this.errorMessage = error.message ?? String(error);
			this.status(`failed: ${this.errorMessage}`, 'fail');
		}
	}

	/** One variant's payload folder, '' when the bundle has a single root. */
	variantBase(dir) {
		return dir ? `${this.base}${dir}/` : this.base;
	}

	/** Selects a variant and reboots the machine from its payload. */
	async selectVariant(dir) {
		if (dir === this.selectedVariant && this.booted) return;
		this.selectedVariant = dir;
		this.halt();
		this.state = 'loading';
		await this.bootVariant(dir, this.selectedExe);
	}

	/**
	 * Selects one executable of a split install (one per language, say) and
	 * reboots. A split install ships every build in one payload, each reading
	 * its own suffixed resources, so the choice is which exe boots.
	 */
	async selectExe(name) {
		if (name === this.selectedExe && this.booted) return;
		this.selectedExe = name;
		this.halt();
		this.state = 'loading';
		await this.bootVariant(this.selectedVariant, name);
	}

	async bootVariant(dir, requestedExe) {
		const base = this.variantBase(dir);
		this.status('loading the bundle…');
		const [artifact, fileList] = await Promise.all([
			this.fetchJson(`${base}artifact.json`),
			this.fetchJson(`${base}game/files.json`)
		]);
		this.artifact = artifact;
		this.saves = Array.isArray(this.bundle?.saves) ? this.bundle.saves : null;
		this.executables = (Array.isArray(artifact.executables) ? artifact.executables : []).map(
			(entry) => entry.name
		);

		// The module is fetched keyed on its build stamp, so a rebuilt payload
		// cannot be missed — not even one whose size did not move.
		const build = artifact.module?.build ?? artifact.module?.bytes ?? 0;
		const wasmResponse = await fetch(`${base}host_wasm.wasm?v=${build}`, { cache: 'no-cache' });
		if (!wasmResponse.ok) throw new Error(`host_wasm.wasm: HTTP ${wasmResponse.status}`);
		const wasmBytes = await wasmResponse.arrayBuffer();

		const { instance } = await WebAssembly.instantiate(wasmBytes, {});
		const host = { ...instance.exports };
		// A wasm i32 result arrives signed; every pointer export is widened once.
		for (const name of POINTER_EXPORTS) {
			const raw = host[name];
			if (typeof raw === 'function') host[name] = (...args) => raw(...args) >>> 0;
		}
		this.host = host;
		// A fresh module starts its counters at zero; the stale frame-version
		// and flip bookkeeping of the previous variant must not suppress its
		// first present.
		this.flips = -1;
		this.version = -1;
		this.paletteVersion = -1;

		this.status('fetching the game…');
		const entries = [];
		for (const item of fileList) {
			const response = await fetch(
				`${base}game/${item.name.split('/').map(encodeURIComponent).join('/')}`,
				{ cache: 'no-store' }
			);
			if (!response.ok) throw new Error(`HTTP ${response.status} for game/${item.name}`);
			entries.push({ name: item.name, bytes: new Uint8Array(await response.arrayBuffer()) });
		}
		this.entries = entries;

		const executables = entries
			.filter((entry) => entry.name.toLowerCase().endsWith('.exe') && !entry.name.includes('/'))
			.map((entry) => entry.name);
		const exeName =
			requestedExe && executables.includes(requestedExe)
				? requestedExe
				: (artifact.default ?? executables[0]);
		if (!exeName) throw new Error('the payload names no executable');
		this.exeName = exeName;
		this.selectedExe = exeName;

		const executable = entries.find((entry) => entry.name === exeName);
		// The screen, the Windows version and the name are read *at* load, so
		// all three come first — the kit's loader contract. A screen set after
		// wh_load still sizes the framebuffer but leaves the guest's display
		// plumbing (a DirectDraw palette, for one) wired against the default.
		const screen = artifact.screen ?? { width: 640, height: 480, depth: 8 };
		if (host.wh_set_screen(screen.width, screen.height, screen.depth) !== 0) {
			throw new Error(this.hostMessage());
		}
		const windows = artifact.windows;
		if (windows && typeof host.wh_set_windows === 'function') {
			// A module older than the export answers with its own default instead.
			host.wh_set_windows(windows.major, windows.minor, windows.build, windows.platform);
		}
		const [namePointer, nameLength] = this.sendInto(new TextEncoder().encode(exeName));
		if (host.wh_set_exe_name(namePointer, nameLength) !== 0) throw new Error(this.hostMessage());
		const [imagePointer, imageLength] = this.sendInto(executable.bytes);
		if (host.wh_load(imagePointer, imageLength) !== 0) {
			throw new Error(`the executable did not load: ${this.hostMessage()}`);
		}

		this.status('providing files…');
		for (const entry of entries) {
			// No executable is game data: the guest opens it as a process image.
			if (entry.name === exeName) continue;
			this.provideFile(entry.name, entry.bytes);
		}

		this.status('restoring saves…');
		await this.restoreSaves();

		this.booted = true;
		// Boot then run, the way the v86 player does: the guest sits on its own
		// title screen, and the first user gesture anywhere brings audio up.
		this.play();
	}

	/** The scratch buffer, one allocation per send: [pointer, length]. */
	sendInto(bytes) {
		const pointer = this.host.wh_alloc(bytes.length) >>> 0;
		new Uint8Array(this.host.memory.buffer, pointer, bytes.length).set(bytes);
		return [pointer, bytes.length];
	}

	hostMessage() {
		const host = this.host;
		if (!host) return '';
		const pointer = host.wh_message_ptr();
		const length = host.wh_message_len();
		if (!pointer || !length) return '';
		return new TextDecoder().decode(new Uint8Array(host.memory.buffer, pointer, length));
	}

	provideFile(name, bytes) {
		const encoded = new TextEncoder().encode(name);
		const pointer = this.host.wh_alloc(encoded.length + bytes.length) >>> 0;
		const memory = new Uint8Array(this.host.memory.buffer);
		memory.set(encoded, pointer);
		if (bytes.length) memory.set(bytes, pointer + encoded.length);
		const result = this.host.wh_provide_file(
			pointer,
			encoded.length,
			bytes.length ? pointer + encoded.length : 0,
			bytes.length
		);
		if (result !== 0) throw new Error(`the host refused ${name}: ${this.hostMessage()}`);
	}

	/** The account's and this browser's save files, replayed before the guest runs. */
	async restoreSaves() {
		const keys = await listSaves(this.slug);
		for (const key of keys) {
			if (!keptBy(this.saves, key)) continue;
			const bytes = await loadSaveFile(this.slug, key);
			if (!bytes) continue;
			const encoded = new TextEncoder().encode(key);
			const pointer = this.host.wh_alloc(encoded.length + bytes.length) >>> 0;
			const memory = new Uint8Array(this.host.memory.buffer);
			memory.set(encoded, pointer);
			memory.set(bytes, pointer + encoded.length);
			this.host.wh_provide_saved(pointer, encoded.length, pointer + encoded.length, bytes.length);
		}
		const restored = this.host.wh_saved_count?.() ?? 0;
		if (restored) this.onDiagnostic(`restored ${restored} save file(s)`);
	}

	/** The guest's files as they stand, copied out of module memory. */
	savedFiles() {
		const host = this.host;
		const files = [];
		const count = host.wh_saved_count();
		for (let index = 0; index < count; index += 1) {
			const nameLength = host.wh_saved_name(index);
			if (!nameLength) continue;
			const name = this.hostMessage();
			const length = host.wh_saved_len(index);
			const pointer = host.wh_saved_ptr(index);
			files.push({
				name,
				bytes: length
					? new Uint8Array(host.memory.buffer, pointer, length).slice()
					: new Uint8Array(0)
			});
		}
		return files;
	}

	/**
	 * Copies the guest's writes into the site's save store, at most once a
	 * second while running, and only for the files the bundle names.
	 */
	persistSaves(force = false) {
		const version = this.host.wh_saved_version();
		if (!force && version === this.savedVersion) return;
		if (!force && performance.now() - this.persistedAt < 1000) return;
		this.savedVersion = version;
		this.persistedAt = performance.now();
		const files = this.savedFiles().filter((file) => keptBy(this.saves, file.name));
		const now = new Map();
		for (const file of files) {
			now.set(file.name, file.bytes);
			const before = this.persisted.get(file.name);
			if (before && sameBytes(before, file.bytes)) continue;
			saveSaveFile(this.slug, file.name, file.bytes).catch((error) => {
				this.onDiagnostic(`save ${file.name}: ${error.message ?? error}`);
			});
		}
		// A name that vanished is a save the guest deleted.
		for (const name of this.persisted.keys()) {
			if (!now.has(name)) {
				deleteSaveFile(this.slug, name).catch(() => {});
			}
		}
		this.persisted = now;
	}

	// ---- presentation -------------------------------------------------------

	refreshPalette() {
		const host = this.host;
		const version = host.wh_palette_version();
		if (version === this.paletteVersion) return false;
		this.paletteVersion = version;
		for (let index = 0; index < 256; index += 1) {
			const rgb = host.wh_palette(index);
			// Canvas wants 0xAABBGGRR in little-endian byte order.
			this.palette[index] =
				(0xff000000 | ((rgb & 0xff) << 16) | (rgb & 0xff00) | ((rgb >> 16) & 0xff)) >>> 0;
		}
		return true;
	}

	draw() {
		const host = this.host;
		const canvas = this.canvas;
		const context = canvas.getContext('2d', { alpha: false });
		const offset = host.wh_framebuffer_offset();
		const width = host.wh_framebuffer_width();
		const height = host.wh_framebuffer_height();
		if (!offset || !width || !height) return;

		if (canvas.width !== width || canvas.height !== height) {
			canvas.width = width;
			canvas.height = height;
			this.image = context.createImageData(width, height);
		}
		if (!this.image) this.image = context.createImageData(width, height);

		// `wh_framebuffer_offset` is a guest address relative to the image base;
		// the host's memory does not start at offset 0 of the module's memory.
		const base = host.wh_memory_ptr();
		const pitch = host.wh_framebuffer_pitch() || width;
		const memory = new Uint8Array(host.memory.buffer);
		const target = new Uint32Array(this.image.data.buffer);
		// The depth is the surface the guest actually draws on, not the one the
		// artifact seeded: Cave Story seeds 8 and runs 16, and a depth read as
		// another one reads palette indices out of pixel pairs — black, all of it.
		const depth = host.wh_framebuffer_depth ? host.wh_framebuffer_depth() : 8;
		if (depth === 8) {
			this.refreshPalette();
			for (let row = 0; row < height; row += 1) {
				const from = row * pitch;
				const to = row * width;
				for (let column = 0; column < width; column += 1) {
					target[to + column] = this.palette[memory[base + offset + from + column]];
				}
			}
		} else if (depth === 16) {
			for (let row = 0; row < height; row += 1) {
				const rowStart = base + offset + row * pitch;
				const to = row * width;
				// Five and six bit channels widen by repeating their top bits,
				// which is how a display expands them: shifting alone caps white
				// at 248,252,248, so every bright pixel comes out a shade dull.
				for (let column = 0; column < width; column += 1) {
					const at = rowStart + column * 2;
					const value = memory[at] | (memory[at + 1] << 8);
					const red = (value >> 11) & 0x1f;
					const green = (value >> 5) & 0x3f;
					const blue = value & 0x1f;
					target[to + column] =
						0xff000000 |
						(((blue << 3) | (blue >> 2)) << 16) |
						(((green << 2) | (green >> 4)) << 8) |
						((red << 3) | (red >> 2));
				}
			}
		} else {
			for (let row = 0; row < height; row += 1) {
				const rowStart = base + offset + row * pitch;
				const to = row * width;
				for (let column = 0; column < width; column += 1) {
					const at = rowStart + column * 4;
					// Little-endian BGRX, which is SDL's 32-bit XRGB surface.
					target[to + column] =
						0xff000000 | (memory[at] << 16) | (memory[at + 1] << 8) | memory[at + 2];
				}
			}
		}
		context.putImageData(this.image, 0, 0);
	}

	// ---- audio --------------------------------------------------------------

	drainAudio() {
		const host = this.host;
		const count = host.wh_audio_event_count();
		if (!count) return;
		const events = new DataView(
			host.memory.buffer,
			host.wh_audio_events_ptr(),
			count * AUDIO_EVENT_BYTES
		);
		const plays = [];
		for (let index = 0; index < count; index += 1) {
			const at = index * AUDIO_EVENT_BYTES;
			const kind = events.getUint32(at, true);
			const voice = events.getUint32(at + 4, true);
			const channels = events.getUint32(at + 8, true);
			const rate = events.getUint32(at + 12, true);
			const frames = events.getUint32(at + 16, true);
			const flags = events.getUint32(at + 28, true);
			const bytes = events.getUint32(at + 32, true);
			const frequency = events.getUint32(at + 36, true);
			const bits = events.getUint32(at + 40, true);
			if (kind === AUDIO_EVENT_PLAY) {
				plays.push({ voice, channels, rate, frames, flags, bytes, frequency, bits });
			} else if (kind === AUDIO_EVENT_DATA) {
				this.onDiagnostic('audio data event seen');
			} else if (kind === AUDIO_EVENT_STOP || kind === AUDIO_EVENT_RELEASE) {
				const playing = this.voices.get(voice);
				if (playing) {
					try {
						playing.stop();
					} catch {
						/* already stopped */
					}
					this.voices.delete(voice);
				}
			}
		}
		// The PCM pointers are only valid until the queue is cleared, so copy first.
		const copied = plays.map((play) => ({
			...play,
			pcm: new Uint8Array(host.memory.buffer, host.wh_audio_pcm_ptr(play.voice), play.bytes).slice()
		}));
		host.wh_audio_events_clear();
		for (const play of copied) this.startVoice(play);
	}

	startVoice(play) {
		const audio = this.audioEnabled ? this.audio : null;
		if (!audio || !play.pcm.length) return;
		const { channels, rate, frames, bits, frequency, flags } = play;
		if (!channels || !rate || !frames) return;
		try {
			const buffer = audio.createBuffer(channels, frames, frequency || rate);
			for (let channel = 0; channel < channels; channel += 1) {
				const out = buffer.getChannelData(channel);
				for (let frame = 0; frame < frames; frame += 1) {
					if (bits === 16) {
						const at = (frame * channels + channel) * 2;
						const value = ((play.pcm[at] | (play.pcm[at + 1] << 8)) << 16) >> 16;
						out[frame] = value / 32768;
					} else {
						const at = frame * channels + channel;
						out[frame] = (play.pcm[at] - 128) / 128;
					}
				}
			}
			const source = audio.createBufferSource();
			source.buffer = buffer;
			source.loop = (flags & 1) !== 0;
			source.connect(audio.destination);
			source.start();
			const previous = this.voices.get(play.voice);
			if (previous) {
				try {
					previous.stop();
				} catch {
					/* already stopped */
				}
			}
			this.voices.set(play.voice, source);
		} catch {
			// A sound the browser refuses is not worth stopping the game for.
		}
	}

	// ---- the loop -----------------------------------------------------------

	frame() {
		if (!this.running) return;
		try {
			const host = this.host;
			const started = performance.now();
			let outcome = 0;
			// Several chunks of guest work per frame, bounded by a wall-time
			// slice: a single 250k-instruction chunk per rAF gives the guest a
			// fifteenth of the throughput the kit's own loader gives it, and a
			// title screen that needs hundreds of millions of instructions
			// takes half a minute to reach its first present at that pace.
			// Twelve of the ~16.7 ms of a frame go to the guest; the rest keeps
			// the page responsive.
			const CHUNK = 64_000;
			let flipped = false;
			for (let chunk = 0; chunk < 64; chunk += 1) {
				if (performance.now() - started > 12) break;
				host.wh_set_time(performance.now());
				outcome = host.wh_run(CHUNK);
				if (outcome !== 0) break;
				if (host.wh_flips() !== this.flips || host.wh_frame_version() !== this.version) {
					flipped = true;
					break;
				}
			}
			this.drainAudio();
			if (flipped || this.refreshPalette()) {
				this.flips = host.wh_flips();
				this.version = host.wh_frame_version();
				this.draw();
			}

			const message = this.hostMessage();
			if (message && message !== this.lastMessage) {
				this.lastMessage = message;
				if (outcome === 2) {
					this.status(`an import is unanswered: ${message}`, 'fail');
					this.halt();
					return;
				}
				if (outcome === 3) {
					this.status(`fault: ${message}`, 'fail');
					this.halt();
					return;
				}
			}
			if (outcome === 1) {
				this.status('the game quit', 'ok');
				this.halt();
				return;
			}
			this.persistSaves();
		} catch (error) {
			// A throw here would otherwise kill the frame loop silently: the game
			// freezes mid-frame with nothing said anywhere. Say it once.
			this.onDiagnostic(`frame: ${error.message ?? error}`);
			this.status(`fault: ${error.message ?? error}`, 'fail');
			this.halt();
			return;
		}
		this.handle = requestAnimationFrame(() => this.frame());
	}

	/** Starts (or resumes) the guest. The chrome auto-starts after boot. */
	play() {
		if (this.running || !this.booted) return;
		this.running = true;
		this.state = 'running';
		this.lastMessage = '';
		this.handle = requestAnimationFrame(() => this.frame());
		this.status('running', 'ok');
	}

	/** The v86-chrome entry point: boot if needed, then run. */
	async start() {
		if (this.booted) {
			this.play();
			return;
		}
		await this.mount(this.canvas);
		this.play();
	}

	/** Fullscreen on the shell element the chrome binds. */
	async toggleFullscreen() {
		if (!this.shell) return;
		if (document.fullscreenElement) {
			await document.exitFullscreen();
		} else {
			await this.shell.requestFullscreen();
		}
	}

	/** Full reboot: a fresh module instance, same bundle and build. */
	async restart() {
		this.halt();
		this.state = 'loading';
		this.status('restarting…');
		await this.bootVariant(this.selectedVariant, this.selectedExe);
		this.play();
	}

	/** Forces the save mirror now, past the throttle window. */
	async saveNow() {
		if (!this.booted || !this.host) return;
		this.persistSaves(true);
	}

	/**
	 * Deletes every save file of this game — the browser's copy and the
	 * account's — and reboots, because the guest already holds the old files
	 * in its memory and would write them straight back.
	 */
	async clearNow() {
		const keys = await listSaves(this.slug);
		for (const key of keys) {
			await deleteSaveFile(this.slug, key);
		}
		this.persisted = new Map();
		this.savedVersion = this.host?.wh_saved_version?.() ?? 0;
		await this.restart();
	}

	// ---- input ----------------------------------------------------------------

	/** PC set 1 scancode for a `KeyboardEvent.code`, or 0 when unmapped. */
	static scancode(code) {
		return KEY_CODES[code] ?? 0;
	}

	/** Keys the page should keep for the game rather than the browser. */
	captures(code) {
		return CAPTURED.has(code);
	}

	key(code, down) {
		const dik = KEY_CODES[code];
		if (!dik || !this.host) return false;
		this.host.wh_key(dik, down ? 1 : 0);
		return true;
	}

	/**
	 * F8+F9 held together toggles fullscreen, the combo every launcher on the
	 * site answers. Returns true when the combo fired and the page should keep
	 * the keys for itself.
	 */
	#pressedKeys = new Set();
	#fullscreenComboLatched = false;
	f8f9(event) {
		if (event.type === 'keydown') this.#pressedKeys.add(event.code);
		else this.#pressedKeys.delete(event.code);
		const combo = this.#pressedKeys.has('F8') && this.#pressedKeys.has('F9');
		if (combo && !this.#fullscreenComboLatched) {
			this.#fullscreenComboLatched = true;
			this.toggleFullscreen().catch(() => {});
			return true;
		}
		if (!combo) this.#fullscreenComboLatched = false;
		return false;
	}

	/** The pointer in canvas coordinates, sent in the guest's own screen space. */
	pointer(event) {
		if (!this.host || !this.canvas) return;
		const bounds = this.canvas.getBoundingClientRect();
		const x = ((event.clientX - bounds.left) / bounds.width) * this.canvas.width;
		const y = ((event.clientY - bounds.top) / bounds.height) * this.canvas.height;
		this.host.wh_pointer(
			Math.max(0, Math.min(this.canvas.width - 1, Math.round(x))),
			Math.max(0, Math.min(this.canvas.height - 1, Math.round(y)))
		);
	}

	pointerButton(button, down) {
		this.host?.wh_pointer_button(button, down ? 1 : 0);
	}

	pause() {
		if (!this.running) return;
		this.running = false;
		this.state = 'paused';
		cancelAnimationFrame(this.handle);
		this.audio?.suspend().catch(() => {});
		this.status('paused', 'ok');
	}

	halt() {
		this.running = false;
		if (this.handle) cancelAnimationFrame(this.handle);
		this.handle = 0;
		this.voices.forEach((voice) => voice.stop?.());
		this.voices.clear();
		this.audio?.close().catch(() => {});
		this.audio = null;
	}

	unmount() {
		this.halt();
		// The last write of the session may be inside the throttle window.
		if (this.booted && this.host) {
			try {
				this.persistSaves(true);
			} catch {
				/* the guest may be mid-write */
			}
		}
		this.booted = false;
	}
}
