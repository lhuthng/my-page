<script>
	// One navigation item for the sidebar rails — used by both the public
	// NavigationSideBar and the dashboard's DashboardSideBar so hover, selected
	// and label behaviour can never drift apart. `icon` is a component (the
	// shell button icons render `<button><svg/></button>`, the dashboard-only
	// ones a bare `<svg>`; both colorings are covered below). `compact` is the
	// user's manual collapse at `lg` and up: the label hides and the tooltip
	// carries the name, exactly like the below-`lg` rail looks.
	let { icon: Icon, label, href, active = false, compact = false } = $props();
</script>

<li class:compact class={active ? 'selected' : undefined}>
	<a {href} aria-current={active ? 'page' : undefined} title={compact ? label : undefined}>
		<Icon class="w-8 shrink-0 transition-all duration-100" />
		<span class="not-lg:hidden min-w-0 truncate" class:hidden={compact}>{label}</span>
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
	/* The collapsed rail must look exactly like the below-`lg` compact bar —
	   the user's reference. The wide-row padding (`lg:px-2`) is what leaks in
	   at `lg` and shoves every icon past its pill, so compact rows keep the
	   4px padding the compact bar uses. */
	li.compact > a {
		@apply px-1;
	}
	/* The collapsed rail must look exactly like the below-`lg` compact bar —
	   the user's reference. The wide-row padding (`lg:px-2`) is what leaks in
	   at `lg` and shoves every icon past its pill, so compact rows keep the
	   4px padding the tablet bar uses. */
	li.compact > a {
		@apply px-1;
	}
</style>
