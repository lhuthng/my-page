<script>
	let { ...rest } = $props();
</script>

<svg {...rest} viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
	<rect x="3" y="6.5" width="26" height="19" rx="2.5" />
	<path d="m4.5 9 11.5 8.5L27.5 9" />
</svg>

<style>
	rect,
	path {
		fill: none;
		stroke-width: 2.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
</style>
