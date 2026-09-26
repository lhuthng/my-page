<script>
	/**
	 * Small Vietnam-flag chip marking an audiobook cover as a Vietnamese
	 * translation. Absolutely positioned against the nearest `relative`
	 * ancestor, so drop it inside a `reading-cover` (or similar) container and
	 * offset it via the `class` prop, e.g. `class="right-1.5 bottom-1.5"`.
	 */
	let { class: className = '' } = $props();
</script>

<span
	class="pointer-events-none absolute z-12 h-5 aspect-[3/2] overflow-hidden rounded-[3px] border border-black/10 shadow-sm {className}"
	title="Bản dịch tiếng Việt"
	lang="vi"
>
	<svg viewBox="0 0 30 20" class="h-full w-full" role="img" aria-label="Cờ Việt Nam">
		<rect width="30" height="20" fill="#DA251D" />
		<polygon
			points="15,4 16.35,8.14 20.71,8.15 17.18,10.71 18.53,14.85 15,12.29 11.47,14.85 12.82,10.71 9.29,8.15 13.65,8.14"
			fill="#FFFF00"
		/>
	</svg>
</span>
