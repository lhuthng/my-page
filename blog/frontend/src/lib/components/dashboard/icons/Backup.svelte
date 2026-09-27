<script>
	let { ...rest } = $props();
</script>

<svg {...rest} viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
	<path d="m21.33 21.33-5.33-5.33-5.33 5.33" />
	<path d="M16 16v12" />
	<path d="M27.19 24.52A6.67 6.67 0 0 0 24 12h-1.68A10.67 10.67 0 1 0 4 21.73" />
</svg>

<style>
	path {
		fill: none;
		stroke-width: 2.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
</style>
