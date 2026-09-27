<script>
	let { ...rest } = $props();
</script>

<svg {...rest} viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
	<ellipse cx="16" cy="7" rx="11" ry="3.5" />
	<path d="M5 7v18c0 1.9 4.9 3.5 11 3.5s11-1.6 11-3.5V7" />
	<path d="M5 16c0 1.9 4.9 3.5 11 3.5s11-1.6 11-3.5" />
</svg>

<style>
	ellipse,
	path {
		fill: none;
		stroke-width: 2.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
</style>
