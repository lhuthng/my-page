<script>
	let { href, text } = $props();
</script>

<a
	class="inline-flex self-start gap-1 rounded-lg bg-dark py-1 pr-2 pl-1 font-semibold text-white transition-colors duration-150 hover:bg-primary"
	{href}
>
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
