<script>
	let { ...rest } = $props();
</script>

<svg {...rest} viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
	<path d="M5.5 8h21" />
	<path d="M12 8V6a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
	<path d="m8 8 1.1 17.2A2.8 2.8 0 0 0 11.9 28h8.2a2.8 2.8 0 0 0 2.8-2.8L24 8" />
	<path d="M13.5 13v10M18.5 13v10" />
</svg>

<style>
	path {
		fill: none;
		stroke-width: 2.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
</style>
