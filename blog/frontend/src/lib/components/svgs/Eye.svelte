<script>
	let { class: className, slashed = false } = $props();
</script>

<svg
	class={className}
	aria-hidden="true"
	fill="none"
	viewBox="0 0 24 24"
	xmlns="http://www.w3.org/2000/svg"
>
	{#if slashed}
		<path
			d="M3 3l18 18M10.6 10.6a2 2 0 0 0 2.8 2.8M9.9 5.2A10.8 10.8 0 0 1 12 5c5 0 8.5 4.1 9.7 6.2a1.6 1.6 0 0 1 0 1.6 16.3 16.3 0 0 1-2.2 2.9M6.5 6.7a16.4 16.4 0 0 0-4.2 4.5 1.6 1.6 0 0 0 0 1.6C3.5 14.9 7 19 12 19a10.8 10.8 0 0 0 4.3-.9"
			stroke="currentColor"
			stroke-linecap="round"
			stroke-linejoin="round"
			stroke-width="1.8"
		/>
	{:else}
		<path
			d="M2.3 11.2C3.5 9.1 7 5 12 5s8.5 4.1 9.7 6.2a1.6 1.6 0 0 1 0 1.6C20.5 14.9 17 19 12 19s-8.5-4.1-9.7-6.2a1.6 1.6 0 0 1 0-1.6Z"
			stroke="currentColor"
			stroke-linecap="round"
			stroke-linejoin="round"
			stroke-width="1.8"
		/>
		<circle
			cx="12"
			cy="12"
			r="3"
			stroke="currentColor"
			stroke-linecap="round"
			stroke-linejoin="round"
			stroke-width="1.8"
		/>
	{/if}
</svg>
