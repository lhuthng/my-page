<script>
	let { ...rest } = $props();
</script>

<svg {...rest} viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
	<circle cx="11.5" cy="10.5" r="4.5" />
	<path d="M3 27c0-4.7 3.8-8.5 8.5-8.5s8.5 3.8 8.5 8.5" />
	<path d="M21.5 6.4a4.5 4.5 0 0 1 0 8.2" />
	<path d="M23.5 19c3.3 1 5.5 4 5.5 8" />
</svg>

<style>
	circle,
	path {
		fill: none;
		stroke-width: 2.5;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
</style>
