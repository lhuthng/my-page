<script>
	// One navigation item for the sidebar rails — used by both the public
	// NavigationSideBar and the dashboard's DashboardSideBar so hover, selected
	// and label behaviour can never drift apart. `icon` is a component (the
	// shell button icons render `<button><svg/></button>`, the dashboard-only
	// ones a bare `<svg>`; both colorings are covered below).
	let { icon: Icon, label, href, active = false } = $props();
</script>

<li class={active ? 'selected' : undefined}>
	<a {href} aria-current={active ? 'page' : undefined}>
		<Icon class="w-8 transition-all duration-100" />
		<span class="not-lg:hidden min-w-0 truncate">{label}</span>
	</a>
</li>

<style lang="postcss">
	@reference "../../../app.css";

	li {
		@apply w-full rounded-lg bg-background/40 text-dark transition-colors duration-50 hover:bg-background/60;

		& > a {
			@apply flex w-full items-center gap-1 px-1 py-1 no-underline! lg:px-2 [&>button]:fill-dark [&>button]:stroke-dark [&>svg]:fill-dark [&>svg]:stroke-dark;
		}
	}
	li.selected {
		@apply bg-dark text-white hover:bg-dark/90 [&>a>button]:fill-white [&>a>button]:stroke-white [&>a>svg]:fill-white [&>a>svg]:stroke-white;
	}
</style>
