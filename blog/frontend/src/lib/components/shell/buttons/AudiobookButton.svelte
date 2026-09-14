<script>
	let { fill, ...rest } = $props();
</script>

<button {...rest}>
	<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" x="0px" y="0px" {fill}>
		<g>
			<path
				fill-rule="evenodd"
				clip-rule="evenodd"
				d="M12 3C7.58172 3 4 6.58172 4 11V17C4 18.1046 4.89543 19 6 19H7C7.55228 19 8 18.5523 8 18V13C8 12.4477 7.55228 12 7 12H6V11C6 7.68629 8.68629 5 12 5C15.3137 5 18 7.68629 18 11V12H17C16.4477 12 16 12.4477 16 13V18C16 18.5523 16.4477 19 17 19H18C19.1046 19 20 18.1046 20 17V11C20 6.58172 16.4183 3 12 3ZM6 14V17H7V14H6ZM17 14V17H18V14H17Z"
			/>
		</g>
	</svg>
</button>

<style lang="postcss">
	@reference "../../../../app.css";

	button {
		@apply not-disabled:hover:cursor-pointer;
	}
</style>
