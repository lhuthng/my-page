<script>
	let { href, text } = $props();
</script>

<a class="inline-flex gap-1 font-semibold bg-primary rounded-lg p-1 text-white" {href}>
	<svg class="inline h-6 stroke-2" viewBox="10 6 12 20">
		<path
			d="M20 8 L12 16 L20 24"
			fill="none"
			stroke="currentColor"
			stroke-linecap="round"
			stroke-linejoin="round"
		/>
	</svg>
	{text}
</a>
