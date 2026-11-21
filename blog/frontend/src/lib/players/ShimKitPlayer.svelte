<script>
	import { onMount } from 'svelte';
	import GamePlayerChrome from './GamePlayerChrome.svelte';
	import { ShimKitPlayer } from './ShimKitPlayer.svelte.js';

	// A shim-kit payload plays on a canvas the kit drives; this wrapper is only
	// wiring: the shared player chrome above and below the screen, the guest's
	// keyboard and pointer, and reactive mirrors of the player's own fields.
	let { title, slug, payloadUrl, beforeDemoPortal, afterDemoPortal } = $props();

	let canvas = $state();
	let phase = $state('idle');
	let statusText = $state('');
	let diagnostic = $state('');
	let running = $state(false);
	let error = $state('');
	let choices = $state([]);
	let saveMessage = $state('');

	const player = new ShimKitPlayer({
		slug,
		payloadUrl,
		onStatus: (text, kind) => {
			statusText = text;
			if (kind === 'fail') diagnostic = text;
		},
		onDiagnostic: (line) => {
			diagnostic = line;
		}
	});

	function mirror() {
		phase = player.phase;
		running = player.running;
		error = player.state === 'error' ? player.errorMessage || statusText : '';
		choices = (player.executables ?? []).map((name) => ({
			value: name,
			label: exeLabel(name),
			active: name === player.selectedExe,
			disabled: false
		}));
	}

	// The player class is plain JavaScript (testable without Svelte), so its
	// display fields are mirrored here on a light tick — the same shape the
	// v86 component polls its disk-cache counts with.
	$effect(() => {
		const timer = setInterval(mirror, 400);
		return () => clearInterval(timer);
	});

	function exeLabel(name) {
		const suffix = name
			.replace(/\.exe$/i, '')
			.split('-')
			.pop()
			?.toLowerCase();
		const languages = { en: 'English', vi: 'Vietnamese', zh: 'Chinese', ja: 'Japanese' };
		return (suffix && languages[suffix]) || name.replace(/\.exe$/i, '');
	}

	function pickChoice(value) {
		player.selectExe(value);
	}

	function onKey(event, down) {
		if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement)
			return;
		// F8+F9 toggles fullscreen, the same combo the other launchers answer.
		if (down && player.f8f9(event)) {
			event.preventDefault();
			return;
		}
		if (player.key(event.code, down)) event.preventDefault();
	}

	function onPointer(event) {
		player.pointer(event);
	}

	function onPointerDown(event) {
		player.pointer(event);
		player.pointerButton(event.button, true);
	}

	function onPointerUp(event) {
		player.pointerButton(event.button, false);
	}

	onMount(async () => {
		await player.mount(canvas);
		mirror();
		const onKeyDown = (event) => onKey(event, true);
		const onKeyUp = (event) => onKey(event, false);
		window.addEventListener('keydown', onKeyDown);
		window.addEventListener('keyup', onKeyUp);
		canvas?.addEventListener('pointermove', onPointer);
		canvas?.addEventListener('pointerdown', onPointerDown);
		window.addEventListener('pointerup', onPointerUp);
		return () => {
			player.unmount();
			window.removeEventListener('keydown', onKeyDown);
			window.removeEventListener('keyup', onKeyUp);
			canvas?.removeEventListener('pointermove', onPointer);
			canvas?.removeEventListener('pointerdown', onPointerDown);
			window.removeEventListener('pointerup', onPointerUp);
		};
	});
</script>

<GamePlayerChrome
	{title}
	{player}
	{beforeDemoPortal}
	{afterDemoPortal}
	{phase}
	{running}
	{statusText}
	{error}
	onRetry={() => player.start()}
	saveAvailable
	{saveMessage}
	soundEnabled={player.audioEnabled}
	{choices}
	onChoice={pickChoice}
	choiceConfirm="Switching reboots the game into the other language. Save first — unsaved progress is lost. Continue?"
	legend={[
		{ keys: ['Click'], text: 'Send the click to the game' },
		{ keys: ['F8', 'F9'], text: 'Toggle fullscreen' }
	]}
	legendNote="Saves are kept in this browser automatically, and mirrored to your account when you are signed in. The save button forces a mirror right now."
>
	<canvas bind:this={canvas} class="h-full w-full object-contain"></canvas>
</GamePlayerChrome>

{#if diagnostic}
	<div class="px-1 py-1 font-mono text-xs text-dark/40" title={diagnostic}>{diagnostic}</div>
{/if}

<style>
	canvas {
		image-rendering: pixelated;
	}
</style>
