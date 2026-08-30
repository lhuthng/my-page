<script>
	let { placeholder = 'Search…', value = $bindable(''), onsearch, onclear, class: klass = '' } =
		$props();
</script>

<div
	class="flex items-center gap-2 bg-background/40 rounded-xl border-2 border-dark/10 focus-within:border-dark px-3 py-2 {klass}"
>
	<input
		type="text"
		{placeholder}
		bind:value
		oninput={() => onsearch?.()}
		class="flex-1 bg-transparent text-base placeholder:text-dark/30 outline-none"
	/>
	{#if value}
		<button
			type="button"
			onclick={() => {
				value = '';
				onclear?.();
			}}
			class="text-dark/40 hover:text-dark text-base cursor-pointer"
		>
			✕
		</button>
	{/if}
</div>
