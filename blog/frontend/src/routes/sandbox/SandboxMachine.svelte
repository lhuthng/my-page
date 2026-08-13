<script>
	import { onMount } from 'svelte';
	import { installWheelGuard, wheelBelongsToEmulator } from '$lib/players/wheel-guard.js';

	// Self-contained: the sandbox drives v86 directly rather than going through
	// the project player, which carries saves, variants, launcher CDs and
	// snapshots that a scratch machine has none of.
	let { system, onready } = $props();

	let screen = $state();
	let shell = $state();
	let status = $state('Starting…');
	let error = $state('');
	let paused = $state(false);
	let mips = $state(0);
	// Off by default here: the sandbox is for poking at things, and a guest with
	// a wheel-aware driver installed should still get its scroll events. The
	// project player defaults the other way.
	let blockWheel = $state(false);

	let emulator = null;
	let sampler = null;
	let lastCount = 0;
	let lastAt = 0;
	let busy = false;
	// Held so a reboot can put the same disc back in the drive.
	let disc = null;

	const handleWheel = (event) => {
		if (!blockWheel) return;
		if (!wheelBelongsToEmulator(event, shell)) return;
		event.preventDefault();
		event.stopImmediatePropagation();
	};

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
			script.onerror = () => reject(new Error('Could not load the emulator.'));
			document.head.append(script);
		});

	const start = async () => {
		try {
			await loadRuntime();
			const V86 = window.V86 ?? window.V86Starter;
			if (!V86) throw new Error('Could not load the emulator.');
			const options = {
				wasm_path: '/v86/build/v86.wasm',
				screen: { container: screen, use_graphical_text: false },
				screen_container: screen,
				autostart: true,
				memory_size: 64 * 1024 * 1024,
				vga_memory_size: 8 * 1024 * 1024,
				boot_order: 786,
				bios: { url: '/v86/bios/seabios.bin' },
				vga_bios: { url: '/v86/bios/vgabios.bin' },
				hda: {
					url: system.base_url,
					size: system.base_size_bytes,
					async: true,
					fixed_chunk_size: system.chunk_size_bytes,
					use_parts: true
				},
				acpi: false,
				disable_speaker: false
			};
			// v86 instantiates the CD drive with or without media, so Windows
			// letters it at boot and a disc can go in whenever. On a reboot the
			// disc that was in the drive goes back in at construction.
			if (disc) options.cdrom = { buffer: disc };

			emulator = new V86(options);
			installWheelGuard(emulator, () => blockWheel);
			status = 'Starting…';
			emulator.add_listener?.('emulator-ready', () => {
				status = 'Running';
				lastCount = emulator.get_instruction_counter?.() ?? 0;
				lastAt = performance.now();
				sampler = setInterval(() => {
					const count = emulator?.get_instruction_counter?.();
					if (count == null) return;
					const now = performance.now();
					const delta = count - lastCount;
					if (delta >= 0 && now > lastAt) mips = delta / ((now - lastAt) / 1000) / 1e6;
					lastCount = count;
					lastAt = now;
				}, 1000);
			});
			onready?.({ insertDisc, ejectDisc });
		} catch (cause) {
			error = cause?.message ?? 'The machine could not start.';
		}
	};

	const teardown = async () => {
		if (sampler) clearInterval(sampler);
		sampler = null;
		mips = 0;
		paused = false;
		await emulator?.destroy?.();
		emulator = null;
	};

	// set_cdrom raises medium_changed, which is what makes Windows re-read the
	// drive without a reboot. Swapping ejects first and waits: dropping a new
	// disc onto an occupied drive only flips that flag, so a guest partway
	// through reading the old disc can miss the change and get a mix of both.
	const insertDisc = async (buffer) => {
		if (!emulator) throw new Error('Start the machine first.');
		if (disc) {
			emulator.eject_cdrom?.();
			await new Promise((resolve) => setTimeout(resolve, 1000));
		}
		disc = buffer;
		await emulator.set_cdrom?.({ buffer });
	};

	const ejectDisc = async () => {
		if (!emulator) return;
		// Not set_cdrom(null): v86's setter early-returns on a falsy buffer, so
		// that call is silently a no-op. eject() is what clears the drive and
		// raises the interrupt.
		emulator.eject_cdrom?.();
		disc = null;
	};

	const reboot = async () => {
		if (busy) return;
		busy = true;
		status = 'Restarting…';
		// Full rebuild rather than v86's restart(): that only resets the CPU and
		// leaves the disk write caches half-written.
		await teardown();
		await start();
		busy = false;
	};

	const pause = async () => {
		if (!emulator) return;
		if (paused) {
			emulator.run?.();
			paused = false;
			status = 'Running';
		} else {
			await emulator.stop?.();
			paused = true;
			status = 'Paused';
		}
	};

	const captureMouse = () => {
		if (typeof emulator?.lock_mouse === 'function') emulator.lock_mouse();
		else screen?.querySelector('canvas')?.requestPointerLock?.();
	};

	const fullscreen = () =>
		document.fullscreenElement ? document.exitFullscreen?.() : shell?.requestFullscreen?.();

	onMount(() => {
		window.addEventListener('wheel', handleWheel, { passive: false, capture: true });
		start();
		return () => {
			window.removeEventListener('wheel', handleWheel, { capture: true });
			teardown();
		};
	});
</script>

<div class="overflow-hidden rounded-xl bg-dark drop-shadow-xl">
	<div class="flex flex-wrap items-center justify-between gap-3 px-3 py-2 text-xs text-white/60">
		<span>
			{status}
			{#if mips > 0}· {mips.toFixed(0)} MIPS{/if}
		</span>
		<span class="flex flex-wrap gap-3">
			<button class="hover:text-white" onclick={captureMouse}>Capture mouse</button>
			<button class="hover:text-white" onclick={pause}>{paused ? 'Resume' : 'Pause'}</button>
			<button class="hover:text-white" onclick={reboot}>Restart</button>
			<button class="hover:text-white" onclick={fullscreen}>Fullscreen</button>
			<button
				class="hover:text-white"
				title="Old Windows can crash when you scroll. Turn this off if that happens."
				onclick={() => (blockWheel = !blockWheel)}
			>
				Scroll: {blockWheel ? 'off' : 'on'}
			</button>
		</span>
	</div>
	<div class="v86-shell" bind:this={shell}>
		<div class="w-full h-130 screen "  bind:this={screen}></div>
	</div>
	{#if error}
		<p class="px-3 py-2 text-xs text-red-400">{error}</p>
	{/if}
</div>

<style lang="postcss">
	@reference "../../app.css";

	.v86-shell {
		@apply grid touch-none grid-rows-[1fr] overscroll-none bg-dark;
	}

	.screen {
		@apply *:mx-auto;
	}

	.v86-shell:fullscreen {
		@apply h-dvh max-h-dvh w-dvw max-w-dvw place-items-center bg-dark;
	}

	.v86-shell:fullscreen .screen {
		@apply aspect-auto h-dvh max-h-dvh w-dvw;
	}
</style>
