<script>
	let { ...rest } = $props();
</script>

<svg {...rest} viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
	<rect x="2.5" y="10.5" width="27" height="12" rx="6" />
	<path d="M10 14.5v4M8 16.5h4" />
	<path d="M21.5 15v.01M25 18v.01" />
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
