<script>
	let { cellWidth, cellHeight, onclick, children, ...rest } = $props();
</script>

<div {...rest}>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		{onclick}
		class="full grid"
		style:grid-template-columns={`repeat(auto-fit,minmax(${cellWidth},1fr))`}
		style:grid-template-rows={`repeat(auto-fit,${cellHeight})`}
	>
		{@render children?.()}
	</div>
</div>
