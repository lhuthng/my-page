<script>
	let { class: className = '', expanded, duration = '300ms', children } = $props();
</script>

<div
	class={`grid transition-[grid-template-rows] ease-out overflow-hidden ${className}`}
	style:transition-duration={duration}
	style:grid-template-rows={expanded ? '1fr' : '0fr'}
>
	<div class="min-h-0">
		{@render children?.()}
	</div>
</div>
