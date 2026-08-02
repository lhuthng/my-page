<script>
	let { min, max, step, value = $bindable(0), oninput, class: className = '', ...rest } = $props();
</script>

<input type="range" {min} {max} {step} bind:value {oninput} class={className} {...rest} />

<style lang="postcss">
	@reference "../../../app.css";

	input[type='range'] {
		@apply h-5 cursor-pointer appearance-none bg-transparent;
	}

	input[type='range']::-webkit-slider-runnable-track {
		@apply h-1.5 rounded-full;
		background: var(--color-primary);
	}

	input[type='range']::-moz-range-track {
		@apply h-1.5 rounded-full;
		background: var(--color-primary);
	}

	input[type='range']::-webkit-slider-thumb {
		@apply h-3.5 w-3.5 appearance-none rounded-full;
		background: var(--color-background);
		border: 2px solid var(--color-primary);
		margin-top: -0.25rem;
	}

	input[type='range']::-moz-range-thumb {
		@apply h-3.5 w-3.5 rounded-full;
		background: var(--color-background);
		border: 2px solid var(--color-primary);
	}
</style>
