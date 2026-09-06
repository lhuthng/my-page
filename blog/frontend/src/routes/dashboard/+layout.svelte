<script>
	import { page } from '$app/stores';

	let { children } = $props();

	let currentPath = $derived($page.url.pathname);

	// Mirrors the sidebar's groups; shown below lg where the rail is hidden.
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
		{ label: 'Backup & Sync', path: '/dashboard/backup' }
	];

	let navGroups = $derived(
		$page.data.role === 'admin' ? [...groups, { label: 'Admin', items: adminItems }] : groups
	);

	let flatTabs = $derived([
		{ label: 'Overview', path: '/dashboard', exact: true },
		...navGroups.flatMap((g) => g.items)
	]);

	function isActive(tab) {
		return (
			currentPath === tab.path ||
			(!tab.exact && tab.path !== '/dashboard' && currentPath.startsWith(tab.path))
		);
	}
</script>

<svelte:head>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<div class="flex flex-col gap-2 lg:gap-4 pb-8">
	<!-- Pill row (replaces the sidebar below lg) -->
	<nav class="lg:hidden">
		<ul
			class="bg-white rounded-xl p-2 shadow-lg flex gap-1 overflow-x-auto custom-scrollbar whitespace-nowrap"
		>
			{#each flatTabs as tab (tab.path)}
				{@const active = isActive(tab)}
				<li>
					<a
						href={tab.path}
						class="block px-3 py-1.5 rounded-lg text-base font-medium transition-colors no-underline! {active
							? 'bg-dark text-white'
							: 'text-dark hover:bg-background/60'}"
						aria-current={active ? 'page' : undefined}
					>
						{tab.label}
					</a>
				</li>
			{/each}
		</ul>
	</nav>

	{@render children?.()}
</div>
