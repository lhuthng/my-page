<script>
	import SearchButton from '../buttons/SearchButton.svelte';

	let {
		term = $bindable(''),
		selection = $bindable('Post'),
		onsearch = () => {},
		onclear = () => {},
		onselectchange = () => {},
		class: className = ''
	} = $props();
</script>

<div
	class="relative z-10 flex h-10 items-center rounded-xl border-2 transition-colors duration-50 bg-dark border-dark overflow-hidden {className}"
>
	<div class="relative grow h-full grid px-2 bg-white">
		<input
			class="w-full"
			name="search-input"
			placeholder="Search"
			autocomplete="off"
			bind:value={term}
		/>
		<button
			title="close-btn"
			class="absolute right-2 top-1/2 -translate-y-1/2 h-full w-fit"
			onclick={onclear}
		>
			<svg class="h-3/5 stroke-dark" viewBox="0 0 24 24">
				<path
					id="primary"
					d="M19,19,5,5M19,5,5,19"
					style="stroke-linecap: round; stroke-linejoin: round; stroke-width: 2;"
				/>
			</svg>
		</button>
	</div>
	<div class="flex items-center h-full cursor-pointer">
		<div
			class="h-full px-2 border-l-2 rounded-r-lg cursor-pointer bg-white hover:bg-dark text-dark hover:text-white"
		>
			<select
				class="full text-current outline-none cursor-pointer"
				name="search-options"
				bind:value={selection}
				onchange={onselectchange}
			>
				<option>Post</option>
				<option>Tag</option>
				<option>User</option>
			</select>
		</div>
		<div>
			<SearchButton
				class="p-2 w-10 h-full transition-transform duration-50 active:translate-y-0.5"
				fill="white"
				onclick={onsearch}
			/>
		</div>
	</div>
</div>
