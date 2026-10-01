<script>
	import LastUpdated from '$lib/components/dashboard/LastUpdated.svelte';

	let {
		label,
		value,
		/** Optional secondary line under the number, e.g. a freshness stamp. */
		detail = null,
		/** Rendered on the right of the label row, e.g. a link to the full list. */
		actions = null
	} = $props();
</script>

<!--
	The one stat tile the dashboard uses. It was hand-written inline on the
	overview page and again elsewhere with three near-identical type ramps; the
	value leads and the label follows because a tile is scanned by its number
	first.
-->
<section class="flex flex-col gap-1 rounded-xl bg-white p-4">
	<p class="text-3xl font-bold text-dark tabular-nums">{value}</p>
	<div class="flex items-baseline justify-between gap-2">
		<p class="text-base text-dark/60">{label}</p>
		{#if actions}
			{@render actions()}
		{/if}
	</div>
	{#if detail}
		<LastUpdated value={detail} label={false} />
	{/if}
</section>
