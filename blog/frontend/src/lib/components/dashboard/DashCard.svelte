<script>
	let { title = '', count, actions, class: klass = '', children, ...rest } = $props();
</script>

<section class="bg-white rounded-xl p-4 flex flex-col gap-4 {klass}" {...rest}>
	{#if title || actions}
		<header class="flex flex-wrap items-center justify-between gap-2">
			{#if title}
				<h2 class="text-2xl font-semibold">
					{title}{#if count !== undefined}<span class="text-dark/40 text-lg font-normal">
							({count})
						</span>{/if}
				</h2>
			{/if}
			{#if actions}
				{@render actions()}
			{/if}
		</header>
	{/if}
	{@render children?.()}
</section>
