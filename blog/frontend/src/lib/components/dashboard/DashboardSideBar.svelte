<script>
	import { page } from '$app/stores';

	// The dashboard's own navigation rail, rendered by the root layout in the
	// same slot the public NavigationSideBar occupies on non-dashboard routes.
	// Grouped so 13+ sections read at a glance; hidden below lg where the
	// dashboard layout shows a scrollable pill row instead.
	let currentPath = $derived($page.url.pathname);

	const groups = [
		{
			label: 'Content',
			items: [
				{ label: 'Posts', path: '/dashboard/posts' },
				{ label: 'Projects', path: '/dashboard/projects', exact: true },
				{ label: 'Games', path: '/dashboard/games', exact: true },
				{ label: 'Series', path: '/dashboard/series' },
				{ label: 'Media', path: '/dashboard/media/manager' }
			]
		},
		{
			label: 'Site',
			items: [
				{ label: 'Users', path: '/dashboard/users' },
				{ label: 'Newsletter', path: '/dashboard/newsletter' },
				{ label: 'Trash', path: '/dashboard/trash' }
			]
		}
	];

	const adminItems = [
		{ label: 'Highlight Posts', path: '/dashboard/highlights' },
		{ label: 'Highlight Projects', path: '/dashboard/projects/highlights' },
		{ label: 'v86 Systems', path: '/dashboard/v86-systems' },
		{ label: 'Database', path: '/dashboard/database' },
		{ label: 'Backup', path: '/dashboard/backup' }
	];

	let navGroups = $derived(
		$page.data.role === 'admin' ? [...groups, { label: 'Admin', items: adminItems }] : groups
	);

	function isActive(tab) {
		return (
			currentPath === tab.path ||
			(!tab.exact && tab.path !== '/dashboard' && currentPath.startsWith(tab.path))
		);
	}
</script>

<nav class="hidden lg:block sticky self-start top-32 mb-4 w-46 min-w-46 drop-shadow-sm">
	<ul class="bg-white p-2 rounded-xl space-y-1">
		<li>
			<a
				href="/dashboard"
				class="flex w-full px-2 py-1.5 rounded-lg font-medium text-dark transition-colors duration-50 no-underline! {currentPath ===
				'/dashboard'
					? 'bg-dark text-white hover:bg-dark/90'
					: 'bg-background/40 hover:bg-background/60'}"
			>
				Overview
			</a>
		</li>
		{#each navGroups as group (group.label)}
			<li class="px-2 pt-3 pb-1 text-xs font-semibold text-dark/40 uppercase tracking-wide">
				{group.label}
			</li>
			{#each group.items as tab (tab.path)}
				{@const active = isActive(tab)}
				<li>
					<a
						href={tab.path}
						class="flex w-full px-2 py-1.5 rounded-lg text-dark transition-colors duration-50 no-underline! {active
							? 'bg-dark text-white hover:bg-dark/90'
							: 'bg-background/40 hover:bg-background/60'}"
						aria-current={active ? 'page' : undefined}
					>
						{tab.label}
					</a>
				</li>
			{/each}
		{/each}
	</ul>
</nav>
