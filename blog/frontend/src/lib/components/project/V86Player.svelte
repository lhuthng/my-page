<script>
	import { onMount } from 'svelte';
	import Portal from '$lib/components/shell/Portal.svelte';
	import { V86Player } from '$lib/players/V86Player.svelte.js';

	let { title, runtime, beforeDemoPortal, afterDemoPortal } = $props();
	const player = new V86Player({ runtime });

	onMount(() => {
		player.mount();
		return () => player.unmount();
	});
</script>

<Portal target={beforeDemoPortal} class="text-dark/75 text-base w-full flex flex-col">
	<span>Status: {player.status}</span>
	<span>Click the game to capture the mouse.</span>
	<span>
		Press <kbd>Esc</kbd>
		to release it.
	</span>
	<span>
		Press <kbd>F8</kbd>
		+
		<kbd>F9</kbd>
		together to toggle fullscreen.
	</span>
</Portal>

<Portal target={afterDemoPortal}>
	<div>
		<label class="flex items-center gap-2 mt-1 text-dark/75">
			<input
				type="checkbox"
				bind:checked={player.disableMouseWheel}
				class="accent-primary h-4 w-4"
			/>
			<span
				class="underline decoration-dashed underline-offset-2 cursor-help"
				title="Wheel could break some games. So I disabled it, but it's not guaranteed, especially you do a Giga Scroll."
			>
				Disable mousewheel
			</span>
		</label>
		<label class="flex items-center gap-2 mt-1 text-dark/75">
			<span>Noise filter:</span>
			<input
				type="range"
				min="0"
				max="200"
				step="10"
				bind:value={player.noiseReductionStrength}
				oninput={() => player.applyNoiseFilter().catch(() => {})}
				class="w-24"
			/>
			<span class="tabular-nums w-14">
				{player.noiseReductionStrength <= 0
					? 'None'
					: player.noiseReductionStrength <= 80
						? 'Light'
						: player.noiseReductionStrength <= 140
							? 'Medium'
							: 'Strong'}
			</span>
		</label>
		<label class="flex items-center gap-2 mt-1 text-dark/75">
			<span>Mouse sensitivity:</span>
			<input
				type="range"
				min="0.05"
				max="1"
				step="0.05"
				bind:value={player.mouseSensitivity}
				class="w-24"
			/>
			<span class="tabular-nums w-8">{player.mouseSensitivity.toFixed(2)}</span>
		</label>
	</div>
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
		@apply font-bold;
	}
	.v86-shell {
		grid-template-rows: 1fr;
		overscroll-behavior: none;
		touch-action: none;
	}
	.screen {
		display: grid;
		place-items: center;
		overflow: hidden;
	}
	.screen :global(canvas) {
		display: block !important;
		min-width: 0;
		width: 100% !important;
		min-height: 0;
		height: 100% !important;
		object-fit: contain !important;
		image-rendering: auto !important;
	}
	.v86-shell:fullscreen {
		width: 100dvw;
		height: 100dvh;
		min-width: 0;
		max-width: 100dvw;
		min-height: 0;
		max-height: 100dvh;
		place-items: center;
		background: #000;
	}
	.v86-shell:fullscreen .screen {
		width: 100dvw;
		height: 100dvh;
	}
	.v86-shell:fullscreen :global(canvas) {
		@apply h-full! max-h-dvh w-full! max-w-dvw object-contain;
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
