<script>
	let { ...rest } = $props();
</script>

<svg {...rest} viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
	<rect x="3" y="5" width="26" height="17" rx="2.5" />
	<path d="M16 22v4.5M10 26.5h12" />
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
