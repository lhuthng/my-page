<script>
	import { onMount } from 'svelte';
	import GamePlayerChrome from './GamePlayerChrome.svelte';
	import { WasmShellBridge, shellEntryUrl } from './WasmShellPlayer.svelte.js';

	// A wasmshell bundle is the game's own page — its shell, its streaming, its
	// boot gate — framed here. The shared chrome wraps it like every other
	// launcher; the bridge mirrors saves, the buttons restart and clear.
	let {
		title,
		slug,
		payloadUrl,
		entry = 'index.html',
		beforeDemoPortal,
		afterDemoPortal
	} = $props();

	let frame = $state();
	let shell = $state();
	let diagnostic = $state('');
	let phase = $state('loading');
	let running = $state(false);
	let error = $state('');

	const src = $derived(
		shellEntryUrl(payloadUrl ?? `/api/games/s/${encodeURIComponent(slug)}/payload/`, entry)
	);
	const bridge = new WasmShellBridge({
		slug,
		onDiagnostic: (line) => {
			diagnostic = line;
		}
	});

	// The frame is its own app; the chrome only knows whether it is alive.
	$effect(() => {
		const timer = setInterval(() => {
			running = Boolean(frame);
			phase = running ? 'running' : 'loading';
		}, 600);
		return () => clearInterval(timer);
	});

	const player = $state({
		// The chrome duck-types the v86 surface: fullscreen and restart act on
		// the frame and the shell around it, saves go through the bridge.
		get shell() {
			return shell;
		},
		async toggleFullscreen() {
			if (!shell) return;
			if (document.fullscreenElement) {
				await document.exitFullscreen();
			} else {
				await shell.requestFullscreen();
			}
		},
		async restart() {
			bridge.restart();
		},
		async saveNow() {
			bridge.post({ type: 'saves:export-request' });
		},
		async clearNow() {
			await bridge.clearNow();
		}
	});

	onMount(() => {
		bridge.attach(frame);
		return () => bridge.detach();
	});
</script>

<GamePlayerChrome
	{title}
	{player}
	{beforeDemoPortal}
	{afterDemoPortal}
	{phase}
	{running}
	statusText={running ? 'Running' : 'Loading the game…'}
	{error}
	saveAvailable
	saveMessage={diagnostic}
	legend={[
		{ keys: ['Click'], text: 'Interact with the game' },
		{ keys: ['F8', 'F9'], text: 'Toggle fullscreen' }
	]}
	legendNote="Saves persist in this browser on their own. Signed in, they also mirror to your account; the save button forces a mirror right now."
>
	<iframe
		bind:this={frame}
		{src}
		title={`${title} game`}
		class="h-full! w-full! border-0 bg-black"
		allow="autoplay; fullscreen"
	></iframe>
</GamePlayerChrome>
