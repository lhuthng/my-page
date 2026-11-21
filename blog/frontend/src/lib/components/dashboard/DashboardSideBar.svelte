<script>
	import { page } from '$app/stores';

	import DashboardButton from '$lib/components/shell/buttons/DashboardButton.svelte';
	import SideBarItem from '$lib/components/shell/SideBarItem.svelte';
	import SidebarToggle from '$lib/components/shell/SidebarToggle.svelte';
	import { navCollapse } from '$lib/dom/nav-collapse.svelte.js';
	import {
		dashboardNavGroups,
		isDashboardTabActive
	} from '$lib/components/dashboard/navigation.js';

	// The dashboard's own navigation rail, rendered by the root layout in the
	// same slot the public NavigationSideBar occupies on non-dashboard routes.
	// Items are the shared SideBarItem (identical hover/selected behaviour as
	// the home rail); hidden below lg where the dashboard sections live in the
	// header's burger menu instead. The rail grows to its content — the page
	// scrolls, the rail never clips or scrolls internally. The SidebarToggle
	// collapses it to the icon rail so the admin pages get the room back.
	let currentPath = $derived($page.url.pathname);

	let navGroups = $derived(dashboardNavGroups($page.data.role));
	let compact = $derived(navCollapse.collapsed);
</script>

<svelte:head>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<nav
	class="sticky-below-header hidden lg:block sticky self-start mb-2 lg:mb-4 w-54 min-w-54 drop-shadow-sm"
	class:collapsed={compact}
>
	<ul class="space-y-2 bg-white p-2 rounded-xl">
		<li class="hidden lg:block"><SidebarToggle /></li>
		<SideBarItem
			icon={DashboardButton}
			label="Overview"
			href="/dashboard"
			active={currentPath === '/dashboard'}
			{compact}
		/>
		{#each navGroups as group (group.label)}
			<li
				class="px-2 pt-1 pb-0.5 text-xs font-semibold text-dark/40 uppercase tracking-wide"
				class:hidden={compact}
			>
				{group.label}
			</li>
			{#each group.items as tab (tab.path)}
				<SideBarItem
					icon={tab.icon}
					label={tab.label}
					href={tab.path}
					active={isDashboardTabActive(currentPath, tab)}
					{compact}
				/>
			{/each}
		{/each}
	</ul>
</nav>

<style lang="postcss">
	@reference "../../../app.css";

	/* The manual collapse: the icon-rail width at every breakpoint; the group
	   labels and item labels hide through the `compact` prop instead. */
	.collapsed {
		width: 3.5rem;
		min-width: 3.5rem;
	}
</style>
