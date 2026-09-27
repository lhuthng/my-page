<script>
	let { ...rest } = $props();
</script>

<svg {...rest} viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
	<rect x="3" y="6" width="26" height="20" rx="3" />
	<circle cx="22" cy="12" r="2" />
	<path d="m3 21.5 6.5-6.5 6 6 3.5-3.5 9 9" />
</svg>

<style>
	rect,
	circle,
	path {
		fill: none;
		stroke-width: 2.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
</style>
