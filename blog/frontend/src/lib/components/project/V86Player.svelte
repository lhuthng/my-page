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
			});
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
		window.addEventListener('keydown', keydown, true);
		window.addEventListener('keyup', keyup, true);
		window.addEventListener('mousemove', handleCapturedMouseMove, true);
		start();
		return () => {
			disposed = true;
			window.removeEventListener('keydown', keydown, true);
			window.removeEventListener('keyup', keyup, true);
			window.removeEventListener('mousemove', handleCapturedMouseMove, true);
			destroy();
		};
	});
</script>

<Portal target={beforeDemoPortal} class="text-dark/75 text-base w-full flex flex-col">
  <span>Status: {status}</span>
	<span>Click the game to capture the mouse.</span>
	<span>Press <kbd>Esc</kbd> to release it. </span>
	<span>Press	<kbd>F8</kbd>+	<kbd>F9</kbd>	together to toggle fullscreen.</span>
</Portal>

<Portal target={afterDemoPortal}>
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
	  @apply w-full! h-full! max-w-dvw max-h-dvh object-contain;
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
