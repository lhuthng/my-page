<script>
	import { formatDateTime, formatLastUpdated, toDateTimeAttr } from '$lib/utils/datetime.js';

	let {
		/** The timestamp to report. Rendered as nothing when absent. */
		value = null,
		/** The field's name. `false` omits it, for a list row that already says so. */
		label = 'Last updated',
		/**
		 * Re-render the relative half on a timer so a list left open does not
		 * keep claiming something changed "just now" hours later.
		 */
		live = false
	} = $props();

	// `Date.now()` is not reactive, so without a nudge the relative half would be
	// frozen at whatever the render happened to be. Reading a tick that only the
	// timer writes keeps the recompute cheap and the intent obvious.
	let tick = $state(0);
	$effect(() => {
		if (!live) return;
		const timer = setInterval(() => (tick += 1), 60_000);
		return () => clearInterval(timer);
	});

	const relative = $derived.by(() => {
		void tick;
		return formatLastUpdated(value);
	});
	const absolute = $derived(formatDateTime(value, { style: 'long' }));
	const machine = $derived(toDateTimeAttr(value));
</script>

{#if relative}
	<!--
		`title` carries the long form so the exact moment is one hover away, while
		the row itself stays scannable: relative first, because that is what
		answers "did this just change?" without being read closely.
	-->
	<span class="inline-flex items-baseline gap-1 text-sm text-dark/50" title={absolute}>
		{#if label}<span>{label}</span>{/if}
		<time datetime={machine}>{relative}</time>
	</span>
{/if}
