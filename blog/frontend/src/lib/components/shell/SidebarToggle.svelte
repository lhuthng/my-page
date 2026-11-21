<script>
	import { navCollapse } from '$lib/dom/nav-collapse.svelte.js';
	import ChevronsLeft from '$lib/components/svgs/ChevronsLeft.svelte';

	// The collapse control both rails render as the first row inside their nav
	// card — a sibling of the items, not a floating box above them, so a
	// collapsed rail reads exactly like the below-`lg` compact rail. Meaningless
	// below `lg` (the responsive classes already compact or hide the rails
	// there), so the row only exists from `lg` up. The chevrons point the way
	// the rail will grow.
	const label = $derived(
		navCollapse.collapsed ? 'Expand the navigation' : 'Collapse the navigation'
	);
</script>

<button
	type="button"
	class="nav-toggle"
	class:collapsed={navCollapse.collapsed}
	aria-label={label}
	aria-pressed={navCollapse.collapsed}
	title={label}
	onclick={() => navCollapse.toggle()}
>
	<ChevronsLeft class="h-5 w-5 {navCollapse.collapsed ? '-scale-x-100' : ''}" />
</button>

<style lang="postcss">
	@reference "../../../app.css";

	/* Same weight and hover as a SideBarItem row, so it reads as part of the
	   list it toggles. */
	button {
		@apply hidden h-10 w-full place-items-center rounded-lg text-dark/60 transition-colors duration-100 hover:bg-background/60 hover:text-dark lg:grid;
	}

	button:focus-visible {
		@apply outline-2 outline-primary;
	}
</style>
