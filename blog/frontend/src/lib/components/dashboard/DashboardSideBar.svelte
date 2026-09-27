<script>
	import { page } from '$app/stores';

	import DashboardButton from '$lib/components/shell/buttons/DashboardButton.svelte';
	import SideBarItem from '$lib/components/shell/SideBarItem.svelte';
	import {
		dashboardNavGroups,
		isDashboardTabActive
	} from '$lib/components/dashboard/navigation.js';

	// The dashboard's own navigation rail, rendered by the root layout in the
	// same slot the public NavigationSideBar occupies on non-dashboard routes.
	// Items are the shared SideBarItem (identical hover/selected behaviour as
	// the home rail); hidden below lg where the dashboard sections live in the
	// header's burger menu instead. The rail grows to its content — the page
	// scrolls, the rail never clips or scrolls internally.
	let currentPath = $derived($page.url.pathname);

	let navGroups = $derived(dashboardNavGroups($page.data.role));
</script>

<svelte:head>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<nav class="hidden lg:block sticky self-start top-32 mb-4 w-54 min-w-54 drop-shadow-sm">
	<ul class="space-y-2 bg-white p-2 rounded-xl">
		<SideBarItem
			icon={DashboardButton}
			label="Overview"
			href="/dashboard"
			active={currentPath === '/dashboard'}
		/>
		{#each navGroups as group (group.label)}
			<li class="px-2 pt-1 pb-0.5 text-xs font-semibold text-dark/40 uppercase tracking-wide">
				{group.label}
			</li>
			{#each group.items as tab (tab.path)}
				<SideBarItem
					icon={tab.icon}
					label={tab.label}
					href={tab.path}
					active={isDashboardTabActive(currentPath, tab)}
				/>
			{/each}
		{/each}
	</ul>
</nav>
