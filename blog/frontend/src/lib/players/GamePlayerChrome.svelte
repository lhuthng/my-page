<script>
	import Portal from '$lib/components/shell/Portal.svelte';
	import Popover from '$lib/components/ui/Popover.svelte';
	import Restart from '$lib/components/svgs/Restart.svelte';
	import Sound from '$lib/components/svgs/Sound.svelte';
	import Zoom from '$lib/components/svgs/Zoom.svelte';
	import { win } from '$lib/dom/windows.svelte.js';

	/**
	 * The player chrome every game launcher shares — the look the v86 player
	 * established, extracted so js-dos, v86, a shim-kit payload and a wasm
	 * source port all read as one thing on the site.
	 *
	 * Three regions, exactly where the v86 player puts them:
	 * - a control bar above the screen (beforeDemoPortal): sound popover,
	 *   fullscreen, restart, save, clear;
	 * - the screen itself: the `screen` snippet inside a fullscreen-able shell;
	 * - context below (afterDemoPortal): choice chips (variants / languages),
	 *   the status line, progress, save messages, and the keys legend.
	 *
	 * The player object is duck-typed to the v86 class's surface: `running`,
	 * `toggleFullscreen()`, `restart()`, `saveNow()`, `clearNow()`,
	 * `applyAudioState()` — so the chrome drives any player that speaks it.
	 */
	let {
		title,
		player,
		beforeDemoPortal,
		afterDemoPortal,
		phase = 'idle',
		statusText = '',
		running = false,
		error = '',
		onRetry = null,
		saveAvailable = false,
		saveMessage = '',
		soundEnabled = null,
		choices = [],
		onChoice = null,
		choiceConfirm = 'Switching will reboot the game — progress in the current session is lost. Continue?',
		legend = [],
		legendNote = '',
		progress = null,
		children
	} = $props();

	let soundOpen = $state(false);
	let clearOpen = $state(false);
	let pendingChoice = $state(null);

	function toggleSound() {
		soundOpen = !soundOpen;
	}
</script>

<Portal target={beforeDemoPortal}>
	<div class="flex flex-row-reverse gap-2 pb-6">
		{#if soundEnabled !== null}
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
							onchange={() => player.applyAudioState?.()}
						/>
						<span>Audio</span>
					</label>
				</div>
			</Popover>
		{/if}

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

		{#if saveAvailable}
			<button
				type="button"
				class="icon-toggle"
				data-color="green"
				aria-label="Save game"
				title="Save game"
				disabled={!running}
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
						data-color="red"
						aria-label="Clear save"
						title="Clear save"
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
					<p class="m-0 text-dark/80">Clear every saved game of this title?</p>
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
	{#if choices.length > 1}
		<Popover bind:open={pendingChoice} position="top" align="left" offset={8} panelClass="min-w-64">
			{#snippet anchor()}
				<div class="flex flex-wrap items-center gap-2" role="group" aria-label="Game variant">
					<span class="font-semibold text-dark/60">Variant:</span>
					{#each choices as choice (choice.value)}
						<button
							type="button"
							class="variant-chip"
							class:active={choice.active}
							disabled={choice.active || choice.disabled}
							onclick={() => (pendingChoice = choice.value)}
						>
							{choice.label}
						</button>
					{/each}
				</div>
			{/snippet}

			<div
				class="flex flex-col gap-3 w-full max-w-lg mx-auto rounded-xl border-2 border-dark bg-white p-4 shadow-lg"
			>
				<p class="m-0 text-dark">
					{choiceConfirm}
				</p>
				<div class="flex justify-end gap-2">
					<div class="duo-btn" data-duo-color="green">
						<button
							type="button"
							onclick={() => {
								const target = pendingChoice;
								pendingChoice = null;
								onChoice?.(target);
							}}
						>
							Switch anyway
						</button>
					</div>
					<div class="duo-btn" data-duo-color="red">
						<button type="button" onclick={() => (pendingChoice = null)}>Hell nah</button>
					</div>
				</div>
			</div>
		</Popover>
	{/if}

	<div class="flex items-center gap-2" aria-live="polite">
		<span class="status-dot {phase}" aria-hidden="true"></span>
		<span class="font-semibold">{error || statusText || 'Idle'}</span>
	</div>

	{#if progress != null}
		<div class="h-2 w-full overflow-hidden rounded-full bg-dark/10">
			<div
				class="h-full rounded-full bg-primary transition-[width]"
				style:width={`${progress}%`}
			></div>
		</div>
	{/if}

	{#if saveMessage}
		<p class="rounded-lg border border-dark/15 bg-dark/5 px-3 py-2 text-dark/80">
			{saveMessage}
		</p>
	{/if}

	{#if legend.length > 0}
		<dl class="player-keys">
			{#each legend as entry (entry.text)}
				<div>
					<dt>
						{#each entry.keys as key, index (key)}
							{#if index > 0}<span>+</span>{/if}
							<kbd>{key}</kbd>
						{/each}
					</dt>
					<dd>{entry.text}</dd>
				</div>
			{/each}
		</dl>
		{#if legendNote}
			<p class="text-dark/60">{legendNote}</p>
		{/if}
	{/if}
</Portal>

<div class="player-shell absolute inset-0 grid bg-black" bind:this={player.shell}>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="screen min-h-0 min-w-0" role="application" aria-label={`${title} game`}>
		{@render children?.()}
	</div>
	{#if error}
		<div class="absolute inset-0 grid place-items-center p-6 text-center">
			<div class="space-y-3">
				<p class="text-accent-red font-semibold">{error}</p>
				{#if onRetry}
					<button class="duo-btn px-3 py-1" onclick={onRetry}>Try again</button>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style lang="postcss">
	@reference '../../app.css';

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

	.variant-chip {
		@apply rounded-full border-2 border-dark/30 bg-white px-3 py-1 text-sm font-semibold text-dark/70 transition-colors duration-100;
	}

	.variant-chip:hover:not(:disabled) {
		@apply border-primary text-primary;
	}

	.variant-chip.active {
		@apply border-dark bg-dark text-white;
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

	.icon-toggle.toggled {
		@apply bg-dark text-white;
		background-color: var(--color-dark);
		color: var(--color-white);
	}

	.icon-toggle:disabled {
		@apply cursor-not-allowed opacity-40;
	}

	.player-shell {
		@apply grid touch-none grid-rows-[1fr] overscroll-none;
	}

	.screen {
		@apply grid place-items-center overflow-hidden;
	}

	.screen :global(canvas) {
		@apply block! h-full! min-h-0 w-full! min-w-0 object-contain!;
		image-rendering: auto !important;
	}

	.screen :global(iframe) {
		@apply block! h-full! w-full! border-0;
	}

	.player-shell:fullscreen {
		@apply h-dvh max-h-dvh min-h-0 w-dvw max-w-dvw min-w-0 place-items-center bg-black;
	}

	.player-shell:fullscreen .screen {
		@apply h-dvh w-dvw;
	}

	.player-keys {
		@apply grid gap-1;
	}

	.player-keys > div {
		@apply flex items-center gap-2;
	}

	.player-keys dt {
		@apply flex items-center gap-1;
	}

	.player-keys dd {
		@apply m-0 text-dark/75;
	}
</style>
