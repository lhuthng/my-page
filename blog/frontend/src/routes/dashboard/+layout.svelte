<script>
	import { page } from '$app/stores';

	let { children } = $props();

	let currentPath = $derived($page.url.pathname);

	const baseTabs = [
		{ label: 'Overview', path: '/dashboard', exact: true },
		{ label: 'Posts', path: '/dashboard/posts' },
		{ label: 'Projects', path: '/dashboard/projects', exact: true },
		{ label: 'Series', path: '/dashboard/series' },
		{ label: 'Users', path: '/dashboard/users' },
		{ label: 'Newsletter', path: '/dashboard/newsletter' }
	];

	let tabs = $derived(
		$page.data.role === 'admin'
			? [
					...baseTabs,
					{ label: 'Highlight Posts', path: '/dashboard/highlights' },
					{ label: 'Highlight Projects', path: '/dashboard/projects/highlights' },
					{ label: 'v86 Systems', path: '/dashboard/v86-systems' },
					{ label: 'Database', path: '/dashboard/database' },
					{ label: 'Backup', path: '/dashboard/backup' }
				]
			: baseTabs
	);
</script>

<svelte:head>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<div class="flex flex-col gap-2 lg:gap-4 pb-8">
	<nav class="bg-white rounded-xl p-2 flex gap-1">
		{#each tabs as tab}
			{@const active =
				currentPath === tab.path ||
				(!tab.exact && tab.path !== '/dashboard' && currentPath.startsWith(tab.path))}
			<a
				href={tab.path}
				class="px-4 py-1.5 rounded-lg text-lg font-medium transition-colors no-underline! {active
					? 'bg-dark text-white'
					: 'text-dark hover:bg-background/60'}"
			>
				{tab.label}
			</a>
		{/each}
	</nav>
	{@render children?.()}
</div>
