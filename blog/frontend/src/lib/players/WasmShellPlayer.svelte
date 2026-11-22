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
		captureMouse = false,
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
		// `shell` is bound by the chrome (GamePlayerChrome's `bind:this`), so
		// it needs a real setter — a getter alone left the fullscreen button
		// acting on nothing.
		get shell() {
			return shell;
		},
		set shell(value) {
			shell = value;
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

	// The bundle's page answers the F8+F9 combo itself (the embedding player
	// gets no keydowns from a focused frame); the legend only explains it.
	const legend = $derived([
		{ keys: ['Click'], text: 'Interact with the game' },
		...(captureMouse ? [{ keys: ['Esc'], text: 'Release the captured mouse' }] : []),
		{ keys: ['F8', 'F9'], text: 'Toggle fullscreen' }
	]);

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
	{legend}
	legendNote="Saves persist in this browser on their own. Signed in, they also mirror to your account; the save button forces a mirror right now."
>
	<iframe
		bind:this={frame}
		{src}
		title={`${title} game`}
		class="h-full! w-full! border-0 bg-black"
		allow={`autoplay; fullscreen${captureMouse ? '; pointer-lock' : ''}`}
	></iframe>
</GamePlayerChrome>
