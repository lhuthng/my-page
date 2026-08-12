<script>
	import { onMount } from 'svelte';
	import Portal from '$lib/components/shell/Portal.svelte';
	import Popover from '$lib/components/ui/Popover.svelte';
	import Slider from '$lib/components/ui/Slider.svelte';
	import Mouse from '$lib/components/svgs/Mouse.svelte';
	import Restart from '$lib/components/svgs/Restart.svelte';
	import Sound from '$lib/components/svgs/Sound.svelte';
	import Zoom from '$lib/components/svgs/Zoom.svelte';
	import { V86Player } from '$lib/players/V86Player.svelte.js';
	import { win } from '$lib/dom/windows.svelte.js';

	let { title, runtime, initialVariant = null, beforeDemoPortal, afterDemoPortal } = $props();
	const player = new V86Player({ runtime });

	let soundOpen = $state(false);
	let mouseOpen = $state(false);
	let clearOpen = $state(false);
	let pendingVariant = $state(null);

	let phase = $derived.by(() => {
		if (player.error) return 'error';
		if (!player.status) return 'idle';
		if (player.status === 'Running') return 'running';
		if (player.status.startsWith('Loading')) return 'loading';
		if (player.status.startsWith('Downloading')) return 'downloading';
		return 'booting';
	});

	onMount(() => {
		if (initialVariant) {
			const match = player.variants.find(
				(v) => (v.name || '').toLowerCase() === initialVariant.toLowerCase()
			);
			if (match) player.selectedVariant = match.index;
			const params = new URLSearchParams(window.location.search);
			if (params.has('variant')) {
				params.delete('variant');
				const query = params.toString();
				const url = query ? `${window.location.pathname}?${query}` : window.location.pathname;
				history.replaceState(history.state, '', url);
			}
		}
		player.mount();
		return () => player.unmount();
	});

	function toggleSound() {
		soundOpen = !soundOpen;
		if (soundOpen) mouseOpen = false;
	}

	function toggleMouse() {
		mouseOpen = !mouseOpen;
		if (mouseOpen) soundOpen = false;
	}
</script>

<Portal target={beforeDemoPortal}>
	<div class="flex flex-row-reverse gap-2 pb-6">
		<Popover
			bind:open={soundOpen}
			position="bottom"
			align={win.isLg ? 'center' : 'right'}
			offset={8}
		>
			{#snippet anchor()}
				<button
					type="button"
					class="icon-toggle"
					class:toggled={soundOpen}
					aria-label="Sound settings"
					aria-expanded={soundOpen}
					title="Sound settings"
					onclick={toggleSound}
				>
					<Sound class="h-6 w-6" />
				</button>
			{/snippet}

			<div class="flex min-w-52 flex-col gap-3 bg-white border-dark border-2 p-4 rounded-xl">
				<label class="flex items-center gap-2 text-dark/75">
					<input
						type="checkbox"
						class="accent-primary h-4 w-4"
						bind:checked={player.audioEnabled}
						onchange={() => player.applyAudioState()}
					/>
					<span>Audio</span>
				</label>
				<label class="flex flex-col gap-1 text-dark/75">
					<span class="flex items-center justify-between gap-3">
						<span>Noise filter</span>
						<span class="tabular-nums">
							{player.noiseReductionStrength <= 0
								? 'None'
								: player.noiseReductionStrength <= 80
									? 'Light'
									: player.noiseReductionStrength <= 140
										? 'Medium'
										: 'Strong'}
						</span>
					</span>
					<Slider
						min="0"
						max="200"
						step="10"
						bind:value={player.noiseReductionStrength}
						oninput={() => player.applyNoiseFilter().catch(() => {})}
					/>
				</label>
			</div>
		</Popover>

		<Popover
			bind:open={mouseOpen}
			position="bottom"
			align={win.isLg ? 'center' : 'right'}
			offset={8}
		>
			{#snippet anchor()}
				<button
					type="button"
					class="icon-toggle"
					class:toggled={mouseOpen}
					aria-label="Mouse settings"
					aria-expanded={mouseOpen}
					title="Mouse settings"
					onclick={toggleMouse}
				>
					<Mouse class="h-6 w-6" />
				</button>
			{/snippet}

			<div class="flex min-w-52 flex-col gap-3 bg-white border-dark border-2 p-4 rounded-xl">
				<label class="flex items-center gap-2 text-dark/75">
					<input
						type="checkbox"
						class="accent-primary h-6 w-6"
						bind:checked={player.disableMouseWheel}
					/>
					<span
						class="underline decoration-dashed underline-offset-2 cursor-help"
						title="Wheel could break some games. So I disabled it, but it's not guaranteed, especially you do a Giga Scroll."
					>
						Disable mousewheel
					</span>
				</label>
				<label class="flex flex-col gap-1 text-dark/75">
					<span class="flex items-center justify-between gap-3">
						<span>Mouse sensitivity</span>
						<span class="tabular-nums">{player.mouseSensitivity.toFixed(2)}</span>
					</span>
					<Slider min="0.05" max="1" step="0.05" bind:value={player.mouseSensitivity} />
				</label>
			</div>
		</Popover>

		<button
			type="button"
			class="icon-toggle"
			aria-label="Toggle fullscreen"
			title="Toggle fullscreen"
			onclick={() => player.toggleFullscreen().catch(() => {})}
		>
			<Zoom class="h-6 w-6" />
		</button>

		<button
			type="button"
			class="icon-toggle"
			aria-label="Restart"
			title="Restart"
			onclick={() => player.restart().catch(() => {})}
		>
			<Restart class="h-6 w-6" />
		</button>

		{#if player.saveAvailable}
			<button
				type="button"
				class="icon-toggle"
				class:toggled={false}
				data-color="green"
				aria-label="Save game"
				title="Save game"
				disabled={!player.running || player.saveBusy}
				onclick={() => player.saveNow()}
			>
				<svg
					class="h-6 w-6"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2Z"></path>
					<polyline points="17 21 17 13 7 13 7 21"></polyline>
					<polyline points="7 3 7 8 15 8"></polyline>
				</svg>
			</button>

			<Popover
				bind:open={clearOpen}
				position="bottom"
				align={win.isLg ? 'center' : 'right'}
				offset={8}
			>
				{#snippet anchor()}
					<button
						type="button"
						class="icon-toggle"
						class:toggled={clearOpen}
						data-color="red"
						aria-label="Clear save"
						title="Clear save"
						disabled={player.saveBusy}
						onclick={() => (clearOpen = true)}
					>
						<svg
							class="h-6 w-6"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<polyline points="3 6 5 6 21 6"></polyline>
							<path
								d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
							></path>
							<line x1="10" x2="10" y1="11" y2="17"></line>
							<line x1="14" x2="14" y1="11" y2="17"></line>
						</svg>
					</button>
				{/snippet}

				<div class="flex min-w-56 flex-col gap-3 bg-white border-dark border-2 p-4 rounded-xl">
					<p class="m-0 text-dark/80">Clear your saved game?</p>
					<div class="flex justify-end gap-2">
						<button type="button" class="duo-btn px-3 py-1" onclick={() => (clearOpen = false)}>
							Cancel
						</button>
						<button
							type="button"
							class="duo-btn px-3 py-1"
							onclick={() => {
								clearOpen = false;
								player.clearNow();
							}}
						>
							Clear
						</button>
					</div>
				</div>
			</Popover>
		{/if}
	</div>
</Portal>

<Portal target={afterDemoPortal} class="text-dark text-base w-full flex flex-col gap-3 pt-6">
	{#if player.variants.length > 1}
		<Popover
			bind:open={pendingVariant}
			position="top"
			align="left"
			offset={8}
			panelClass="min-w-64"
		>
			{#snippet anchor()}
				<div class="flex flex-wrap items-center gap-2" role="group" aria-label="Game variant">
					<span class="font-semibold text-dark/60">Variant:</span>
					{#each player.variants as variant (variant.index)}
						<button
							type="button"
							class="variant-chip"
							class:active={player.selectedVariant === variant.index}
							disabled={player.selectedVariant === variant.index}
							onclick={() => (pendingVariant = variant.index)}
						>
							{variant.name || `Variant ${variant.index}`}
						</button>
					{/each}
				</div>
			{/snippet}

			<div
				class="flex flex-col gap-3 w-full max-w-lg mx-auto rounded-xl border-2 border-dark bg-white p-4 shadow-lg"
			>
				<p class="m-0 text-dark">
					Switching will do a <b>soft-reboot</b>
					(no re-download). Your progress will
					<b class="text-accent-red">be lost</b>
					- though a save-sync is attempted first. Continue?
				</p>
				<div class="flex justify-end gap-2">
					<div class="duo-btn" data-duo-color="green">
						<button
							type="button"
							onclick={() => {
								const target = pendingVariant;
								pendingVariant = null;
								player.selectVariant(target);
							}}
						>
							Reboot anyway
						</button>
					</div>
					<div class="duo-btn" data-duo-color="red">
						<button type="button" onclick={() => (pendingVariant = null)}>Hell nah</button>
					</div>
				</div>
			</div>
		</Popover>
	{/if}
	<div class="flex items-center gap-2" aria-live="polite">
		<span class="status-dot {phase}" aria-hidden="true"></span>
		<span class="font-semibold">{player.error || player.status || 'Idle'}</span>
		{#if phase === 'running' && player.diskFetching}
			<span class="disk-fetch-pill">Fetching game data…</span>
		{/if}
	</div>

	{#if player.downloadProgress != null}
		<div class="h-2 w-full overflow-hidden rounded-full bg-dark/10">
			<div
				class="h-full rounded-full bg-primary transition-[width]"
				style:width={`${player.downloadProgress}%`}
			></div>
		</div>
	{/if}

	{#if player.saveMessage}
		<p class="rounded-lg border border-dark/15 bg-dark/5 px-3 py-2 text-dark/80">
			{player.saveMessage}
		</p>
	{/if}

	<dl class="v86-keys">
		<div>
			<dt><kbd>Click</kbd></dt>
			<dd>Capture the mouse</dd>
		</div>
		<div>
			<dt><kbd>Esc</kbd></dt>
			<dd>Release the mouse</dd>
		</div>
		<div>
			<dt>
				<kbd>F8</kbd>
				<span>+</span>
				<kbd>F9</kbd>
			</dt>
			<dd>Toggle fullscreen</dd>
		</div>
	</dl>

	<p
		class="rounded-lg border border-accent-yellow/60 bg-accent-yellow-light-3 px-3 py-2 text-dark/80"
	>
		Avoid scrolling hard while booting or playing - it's fragile and could break the game.
	</p>
	<p class="text-dark/60">
		Saves travel on a virtual floppy: play the game, quit it, then press the save button.
	</p>
</Portal>

<div class="v86-shell absolute inset-0 grid bg-black" bind:this={player.shell}>
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		class="screen min-h-0 min-w-0"
		bind:this={player.screenContainer}
		aria-label={`${title} emulator`}
		role="application"
		tabindex="0"
		onclick={() => player.running && player.captureMouse()}
		onkeydown={(event) => event.key === 'Enter' && player.running && player.captureMouse()}
	>
		<div class="font-mono text-white whitespace-pre"></div>
		<canvas class="hidden"></canvas>
	</div>
	{#if player.error}
		<div class="absolute inset-0 grid place-items-center bg-background p-6 text-center">
			<div class="space-y-3">
				<p class="text-accent-red font-semibold">{player.error}</p>
				<button class="duo-btn px-3 py-1" onclick={() => player.start()}>Try again</button>
			</div>
		</div>
	{/if}
</div>

<style lang="postcss">
	@reference "../../../app.css";

	kbd {
		@apply rounded-md border border-dark/40 bg-dark/10 px-1.5 py-0.5 font-mono text-sm font-bold text-dark;
	}

	.status-dot {
		@apply h-2.5 w-2.5 shrink-0 rounded-full;
	}

	.status-dot.running {
		@apply bg-accent-green;
	}

	.status-dot.error {
		@apply bg-accent-red;
	}

	.status-dot.loading,
	.status-dot.booting,
	.status-dot.downloading {
		@apply animate-pulse bg-accent-yellow;
	}

	.status-dot.idle {
		@apply bg-dark/30;
	}

	.disk-fetch-pill {
		@apply animate-pulse rounded-full bg-accent-yellow-light-3 px-2 py-0.5 text-xs font-normal text-dark/70;
	}

	.v86-keys {
		@apply grid gap-1;
	}

	.v86-keys > div {
		@apply flex items-center gap-2;
	}

	.v86-keys dt {
		@apply flex items-center gap-1;
	}

	.v86-keys dd {
		@apply m-0 text-dark/75;
	}

	.icon-toggle {
		--icon-color: var(--color-dark);
		--icon-toggled-color: var(--color-white);
		--icon-background: var(--color-white);
		--icon-toggled-background: var(--color-dark);

		@apply grid h-9 w-9 place-items-center rounded-lg border-2 border-dark transition-colors duration-100;

		color: var(--icon-color);
		border-color: var(--icon-color);
		background-color: var(--icon-background);
	}

	.icon-toggle[data-color='red'] {
		--icon-color: var(--color-accent-red);
		--icon-toggled-color: var(--color-accent-red-light-4);
		--icon-background: var(--color-accent-red-light-4);
		--icon-toggled-background: var(--color-accent-red);
	}

	.icon-toggle[data-color='green'] {
		--icon-color: var(--color-accent-green);
		--icon-toggled-color: var(--color-accent-green-light-4);
		--icon-background: var(--color-accent-green-light-4);
		--icon-toggled-background: var(--color-accent-green);
	}

	.variant-chip {
		@apply rounded-full border-2 border-dark/30 bg-white px-3 py-1 text-sm font-semibold text-dark/70 transition-colors duration-100;
	}

	.variant-chip:hover:not(:disabled) {
		@apply border-primary text-primary;
	}

	.variant-chip.active {
		@apply border-dark bg-dark text-white;
	}

	.icon-toggle.toggled {
		@apply bg-dark text-white;
		background-color: var(--color-dark);
		color: var(--color-white);
	}

	.icon-toggle:disabled {
		@apply cursor-not-allowed opacity-40;
	}

	.v86-shell {
		@apply grid touch-none grid-rows-[1fr] overscroll-none;
	}

	.screen {
		@apply grid place-items-center overflow-hidden;
	}

	.screen :global(canvas) {
		@apply block! h-full! min-h-0 w-full! min-w-0 object-contain!;
		image-rendering: auto !important;
	}

	.v86-shell:fullscreen {
		@apply h-dvh max-h-dvh min-h-0 w-dvw max-w-dvw min-w-0 place-items-center bg-black;
	}

	.v86-shell:fullscreen .screen {
		@apply h-dvh w-dvw;
	}

	.v86-shell:fullscreen > canvas {
		@apply h-full! max-h-dvh w-full! max-w-dvw object-contain!;
		image-rendering: auto;
	}
</style>
