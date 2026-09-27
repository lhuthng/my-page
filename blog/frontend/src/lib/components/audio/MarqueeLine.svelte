<script>
	// One line that drifts left and right when it is too long for its box, and
	// sits still when it fits. Used for every title on the mini player, so a long
	// book or chapter name can be read in full instead of being cut off.
	//
	// The travel is measured, not guessed: the text is laid out at its natural
	// width, and the excess over the container becomes both the distance it
	// travels (--marquee-shift) and, roughly, the pace (--marquee-duration).
	let { class: className = '', children } = $props();

	let lineWidth = $state(0);
	let textWidth = $state(0);
	const overflow = $derived(Math.max(0, textWidth - lineWidth));
	const marqueeSeconds = $derived(Math.round(6 + overflow / 25));
</script>

<div class="overflow-hidden whitespace-nowrap {className}" bind:clientWidth={lineWidth}>
	<span
		class="inline-block whitespace-nowrap motion-reduce:animate-none"
		class:animate-marquee={overflow > 1}
		style="--marquee-shift: -{overflow}px; --marquee-duration: {marqueeSeconds}s"
		bind:clientWidth={textWidth}
	>
		{@render children()}
	</span>
</div>
