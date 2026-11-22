<script>
	import { fade } from 'svelte/transition';
	import { getContext } from 'svelte';
	import { browser } from '$app/environment';
	import BackButton from '../ui/BackButton.svelte';
	import JsDosPlayer from './JsDosPlayer.svelte';
	import V86Player from './V86Player.svelte';
	import ShimKitPlayer from '$lib/players/ShimKitPlayer.svelte';
	import WasmShellPlayer from '$lib/players/WasmShellPlayer.svelte';
	import InviteGate from '$lib/components/game/InviteGate.svelte';
	import { win } from '$lib/dom/windows.svelte.js';

	// `showBack` and `heading` exist for the embedded form (`:::app game`): an
	// article supplies its own context, and a heading rendered here would also
	// be collected as one of the post's headings by `findHeaders`.
	let {
		title,
		slug,
		demoType = 'html5',
		demoUrl,
		v86Runtime,
		width = '100%',
		height = '520px',
		backHref = '/projects',
		backLabel = 'All projects',
		showBack = true,
		heading = 'Demo',
		accessMode = 'open',
		entitled = true,
		captureMouse = false,
		children,
		...rest
	} = $props();

	let loaded = $state(false);
	let beforeDemoPortal = $state();
	let afterDemoPortal = $state();
	let gated = $derived(accessMode === 'invited' && !entitled);
	let startsOnDemand = $derived(
		['html5', 'embed', 'webgl', 'jsdos', 'v86', 'shimkit', 'wasmshell'].includes(demoType)
	);
	let isGame = $derived(['jsdos', 'v86', 'shimkit', 'wasmshell'].includes(demoType));
	let startLabel = $derived(
		isGame ? 'Start game' : demoType === 'webgl' ? 'Launch WebGL' : 'Start Demo'
	);

	// --- space requests --------------------------------------------------------------------
	//
	// A game the author gave a fixed frame width (`demo_width` in the editor)
	// can outgrow the viewport with the navigation rail expanded. When that is
	// the case — and collapsing the rail actually makes the frame fit — this
	// component claims the compact rail from the layout's context for as long
	// as it is mounted, and says so with a one-click undo. The user's own
	// toggle outranks the request (expanding declines it for this session),
	// and fluid frames (width in %) never ask — they always fit.
	const navRail = getContext('nav-rail') ?? null;

	let bezel = $state();
	let spaceRequested = $state(false);

	const requestId = $derived(`game:${slug}`);
	// A frame in % fills whatever the layout gives and never needs the rail.
	const fixedFrameWidth = $derived(/^\d+(\.\d+)?px$/.test(width) ? parseFloat(width) : null);

	function releaseSpace() {
		if (spaceRequested) {
			navRail?.release(requestId);
			spaceRequested = false;
		}
	}

	function expandBack() {
		navRail?.decline(requestId);
		navRail?.release(requestId);
		spaceRequested = false;
	}

	function measureFrameSpace() {
		if (!navRail || !bezel || fixedFrameWidth == null || !isGame || gated) {
			releaseSpace();
			return;
		}
		// The author's frame is what it is; `max-w-full` means the page never
		// "overflows" — it squeezes the frame below its suggested width
		// instead. That squeeze is the signal: compare the suggestion against
		// the space each rail shape would leave for the content column.
		// Footprints at `lg` and up, gap included: expanded w-46 (184px) + gap
		// 16, compact 3.5rem (56px) + gap 16.
		const fitsExpanded = fixedFrameWidth <= win.width - 200;
		const fitsCompact = fixedFrameWidth <= win.width - 72;
		if (fitsExpanded) {
			// Plenty of room with the rail expanded; the user's preference rules.
			releaseSpace();
			return;
		}
		if (
			fitsCompact &&
			!spaceRequested &&
			!navCollapseUserCollapsed() &&
			!navRail.isDeclined(requestId)
		) {
			if (browser) {
				window.__spaceLog = window.__spaceLog || [];
				window.__spaceLog.push(['request', win.width, fixedFrameWidth]);
			}
			navRail.request(requestId, onDeclined);
			spaceRequested = true;
		}
	}

	function navCollapseUserCollapsed() {
		return navRail.userCollapsed();
	}

	function onDeclined() {
		// The rail's toggle declined this request on the user's behalf; remember
		// it for the session so the page stops claiming the rail.
		navRail?.decline(requestId);
		navRail?.release(requestId);
		spaceRequested = false;
	}

	$effect(() => {
		// Re-measure when the viewport changes or the user flips their own
		// preference; the request lives in this effect's cleanup, so leaving
		// the page hands the rail back.
		win.width;
		navCollapseUserCollapsed();
		measureFrameSpace();
		return () => releaseSpace();
	});
</script>

<section class="bg-white rounded-xl p-4">
	{#if showBack || heading}
		<div class="space-y-2">
			{#if showBack}
				<BackButton href={backHref} text={backLabel} />
			{/if}
			{#if heading}
				<div class="flex items-center gap-3 mb-3">
					<h2 class="text-xl lg:text-2xl">{heading}</h2>
					<hr class="grow border" />
				</div>
			{/if}
		</div>
	{/if}
	<div class="mx-auto max-w-full" bind:this={beforeDemoPortal} style:width></div>

	{#if spaceRequested}
		<!-- The rail was collapsed for this frame; say so, and hand the room
		     back on one click. -->
		<div
			transition:fade={{ duration: 150 }}
			class="mx-auto max-w-full mb-2 flex items-center justify-between gap-3 rounded-xl border border-dark/15 bg-dark/5 px-3 py-2 text-sm text-dark/80"
			style:width
		>
			<span>Collapsed the navigation to give {title} more room.</span>
			<button
				type="button"
				class="shrink-0 font-semibold text-primary underline hover:text-dark"
				onclick={expandBack}
			>
				Expand
			</button>
		</div>
	{/if}

	<!-- Device bezel: the same gradient faceplate over a dark screen as the
	     audio player, so the emulator is framed like the rest of the site
	     instead of sitting in a flat wash. -->
	<div
		class="mx-auto max-w-full rounded-2xl border-2 border-dark/20 bg-gradient-to-b from-primary/25 via-background/45 to-secondary/50 p-2 sm:p-3"
		style:width
		bind:this={bezel}
	>
		<div class="relative overflow-hidden rounded-xl bg-dark shadow-inner" style:height>
			{#if gated}
				<InviteGate {title} {slug} />
			{:else if startsOnDemand && !loaded}
				<div class="absolute inset-0 grid place-items-center">
					<div class="duo-btn" data-duo-color="green">
						<button onclick={() => (loaded = true)}>{startLabel}</button>
					</div>
				</div>
			{:else if demoType === 'jsdos'}
				<div in:fade={{ duration: 250 }} class="h-full w-full">
					<JsDosPlayer {title} bundleUrl={demoUrl} {beforeDemoPortal} {afterDemoPortal} />
				</div>
			{:else if demoType === 'v86'}
				<div in:fade={{ duration: 250 }} class="h-full w-full">
					<V86Player {title} runtime={v86Runtime} {...rest} {beforeDemoPortal} {afterDemoPortal} />
				</div>
			{:else if demoType === 'shimkit'}
				<div in:fade={{ duration: 250 }} class="h-full w-full">
					<ShimKitPlayer {title} {slug} payloadUrl={demoUrl} {beforeDemoPortal} {afterDemoPortal} />
				</div>
			{:else if demoType === 'wasmshell'}
				<div in:fade={{ duration: 250 }} class="h-full w-full">
					<WasmShellPlayer
						{title}
						{slug}
						payloadUrl={demoUrl}
						{captureMouse}
						{beforeDemoPortal}
						{afterDemoPortal}
					/>
				</div>
			{:else if demoType === 'video'}
				<video class="block h-full w-full bg-black object-contain" src={demoUrl} controls {title}>
					<track kind="captions" src="" srclang="en" label="English captions" default />
				</video>
			{:else if demoType === 'download'}
				<div
					class="absolute inset-0 grid place-items-center bg-gradient-to-b from-primary/20 via-background/40 to-secondary/40 p-6 text-center"
				>
					<div class="space-y-4">
						<p class="text-dark/80 text-lg">This project is available for download.</p>
						<div class="duo-btn text-xl" data-duo-color="green">
							<a
								href={demoUrl}
								download
								target="_blank"
								rel="noopener noreferrer"
								class="no-underline! block px-8 py-3"
							>
								Download Project
							</a>
						</div>
					</div>
				</div>
			{:else}
				<iframe
					in:fade={{ duration: 250 }}
					class="block h-full w-full bg-white"
					src={demoUrl}
					title={`${title} demo`}
					scrolling="no"
					frameborder="0"
				></iframe>
			{/if}
		</div>
	</div>

	<div bind:this={afterDemoPortal} class="mx-auto max-w-full" style:width></div>

	{@render children?.()}
</section>
