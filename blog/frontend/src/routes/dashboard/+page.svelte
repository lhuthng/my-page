<script>
	import { onMount, untrack } from 'svelte';
	import { invalidateAll } from '$app/navigation';
	import { gql } from '$lib/api/graphql';
	import DashCard from '$lib/components/dashboard/DashCard.svelte';
	import EmptyState from '$lib/components/dashboard/EmptyState.svelte';
	import LastUpdated from '$lib/components/dashboard/LastUpdated.svelte';
	import PageHeader from '$lib/components/dashboard/PageHeader.svelte';
	import StatCard from '$lib/components/dashboard/StatCard.svelte';

	let { data } = $props();

	let overview = $state(untrack(() => data).overview);
	let loading = $state(untrack(() => data).overview === null);
	// When these figures were read. Not `createdAt`-style data of their own — it
	// is the age of the fetch, which is what tells the reader whether the numbers
	// under it can still be trusted.
	let fetchedAt = $state(new Date());
	let refreshing = $state(false);
	let activeTopTab = $state('views');
	let visitorCountries = $derived(data.visitorCountries ?? []);
	let visitorTotal = $derived(visitorCountries.reduce((sum, item) => sum + item.visits, 0));
	let visitorUnknown = $derived(
		visitorCountries
			.filter((item) => item.country_code === 'XX')
			.reduce((sum, item) => sum + item.visits, 0)
	);
	let topVisitorCountries = $derived(
		Object.values(
			visitorCountries.reduce((map, item) => {
				const current = map[item.country_code] ?? {
					country_code: item.country_code,
					visits: 0
				};
				current.visits += item.visits;
				map[item.country_code] = current;
				return map;
			}, {})
		)
			.sort((a, b) => b.visits - a.visits || a.country_code.localeCompare(b.country_code))
			.slice(0, 8)
	);
	const countryNames = new Intl.DisplayNames(['en'], { type: 'region' });

	let topPosts = $derived(
		activeTopTab === 'views'
			? overview?.topPostsByViews
			: activeTopTab === 'likes'
				? overview?.topPostsByLikes
				: overview?.topPostsByComments
	);

	function roleLabel(role) {
		return role === 'admin' ? 'Admin' : role === 'moderator' ? 'Mod' : 'User';
	}

	function countryLabel(code) {
		if (code === 'XX') return 'Unknown';
		return countryNames.of(code) ?? code;
	}

	onMount(() => {
		// Only when the server could not hand the overview over. Otherwise its
		// numbers are already on screen and the stamp already describes them.
		if (loading) refresh({ initial: true });
	});

	/**
	 * Re-read the overview, moving the freshness stamp with it.
	 *
	 * The country breakdown comes from the page's server load rather than from
	 * this GraphQL query, so a genuine refresh has to re-run both — otherwise the
	 * button would quietly leave that half of the page stale while claiming the
	 * whole page was current.
	 *
	 * Only the initial load blanks the page into skeletons. A manual refresh
	 * leaves the figures up: "these numbers changed" is not a reason to collapse
	 * a layout the author was already reading.
	 */
	async function refresh({ initial = false } = {}) {
		if (initial) loading = true;
		else refreshing = true;
		try {
			const [result] = await Promise.all([gql.overview(), invalidateAll()]);
			overview = result.overview;
		} catch {
			overview = null;
		} finally {
			fetchedAt = new Date();
			refreshing = false;
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Dashboard | Huu Thang's Blog</title>
</svelte:head>

<section class="flex flex-col gap-4 pb-8">
	<PageHeader title="Dashboard">
		{#snippet actions()}
			<div class="flex items-center gap-3">
				<!-- The figures below are a snapshot, so the page says how old the
				     snapshot is and offers the one thing that makes it current again. -->
				<LastUpdated value={fetchedAt} label="Updated" live />
				<div class="w-fit duo-btn" data-duo-color="light" class:opacity-60={refreshing}>
					<button onclick={() => refresh()} disabled={refreshing}>
						{refreshing ? 'Refreshing…' : 'Refresh'}
					</button>
				</div>
			</div>
		{/snippet}
	</PageHeader>

	{#if loading}
		<div class="grid grid-cols-2 xl:grid-cols-4 gap-4">
			{#each { length: 4 } as _, i (i)}
				<div
					class="bg-white rounded-xl p-4 h-20 animate-pulse"
					style:animation-delay={`${i * 100}ms`}
				></div>
			{/each}
		</div>
		<div class="grid xl:grid-cols-3 gap-4">
			<div class="xl:col-span-2 bg-white rounded-xl p-4 h-64 animate-pulse"></div>
			<div class="bg-white rounded-xl p-4 h-64 animate-pulse"></div>
		</div>
	{:else if overview}
		<!-- ── Stat cards ─────────────────────────────── -->
		<div class="grid grid-cols-2 xl:grid-cols-4 gap-4">
			{#each [['Published Posts', overview.totalPublished], ['Drafts', overview.totalDrafts], ['Registered Users', overview.totalUsers], ['Comments', overview.totalComments]] as [label, value]}
				<StatCard {label} {value} />
			{/each}
		</div>

		<!-- ── Top posts + Role breakdown ─────────────── -->
		<div class="grid xl:grid-cols-3 gap-4">
			<!-- Top posts (takes 2/3 width on xl) -->
			<DashCard title="Top Performing Posts" class="xl:col-span-2">
				{#snippet actions()}
					<div class="flex gap-1 bg-background/40 rounded-lg p-1 text-base">
						{#each [['views', 'Views'], ['likes', 'Likes'], ['comments', 'Comments']] as [key, label]}
							<button
								onclick={() => (activeTopTab = key)}
								class="px-3 py-1 rounded-md transition-colors cursor-pointer {activeTopTab === key
									? 'bg-white font-semibold shadow-sm text-dark'
									: 'text-dark/60 hover:text-dark'}"
							>
								{label}
							</button>
						{/each}
					</div>
				{/snippet}
				{#if topPosts?.length}
					<ol class="flex flex-col">
						{#each topPosts as post, i}
							<li
								class="grid grid-cols-[1.5rem_minmax(0,1fr)_auto] items-start gap-3 py-2 first:pt-0 last:pb-0 hover:bg-background/20 transition-colors rounded-lg px-2 -mx-2"
							>
								<span class="text-center text-base font-bold leading-6 text-dark/30 tabular-nums">
									{i + 1}
								</span>
								<div class="min-w-0">
									<a
										href="/posts/{post.slug}"
										class="block truncate text-base font-medium leading-6 hover:text-primary"
									>
										{post.title}
									</a>
									<span class="text-sm text-dark/50">by {post.authorName}</span>
								</div>
								<span class="text-base font-semibold leading-6 text-primary tabular-nums">
									{activeTopTab === 'views'
										? post.views
										: activeTopTab === 'likes'
											? post.likes
											: post.commentsCount}
								</span>
							</li>
						{/each}
					</ol>
				{:else}
					<EmptyState message="No published posts yet" />
				{/if}
			</DashCard>

			<!-- Role breakdown (1/3) -->
			<DashCard title="User Roles">
				{#if overview.roleCounts}
					{@const total = Math.max(
						overview.roleCounts.admin + overview.roleCounts.moderator + overview.roleCounts.user,
						1
					)}
					<div class="flex flex-col gap-3 grow">
						{#each [['Admins', overview.roleCounts.admin, 'bg-accent-red'], ['Moderators', overview.roleCounts.moderator, 'bg-accent-blue'], ['Users', overview.roleCounts.user, 'bg-accent-green']] as [label, count, color]}
							<div class="flex flex-col gap-1">
								<div class="flex justify-between text-base">
									<span class="font-medium">{label}</span>
									<span class="text-dark/60">{count}</span>
								</div>
								<div class="h-1.5 bg-background/50 rounded-full overflow-hidden">
									<div
										class="h-full rounded-full {color} transition-all duration-500"
										style="width:{Math.round((count / total) * 100)}%"
									></div>
								</div>
							</div>
						{/each}
					</div>
					<p class="text-sm text-dark/40 border-t border-background pt-3 text-center">
						{overview.totalUsers} users total
					</p>
				{/if}
			</DashCard>
		</div>

		<!-- ── Growth chart ────────────────────────────── -->
		{#if overview.growth?.length}
			{@const maxVal = Math.max(...overview.growth.flatMap((g) => [g.newPosts, g.newUsers]), 1)}
			<DashCard title="Activity - Last 30 Days">
				<div class="flex gap-4 text-sm text-dark/60">
					<span class="flex items-center gap-1.5">
						<span class="inline-block w-3 h-3 rounded-sm bg-accent-blue"></span>
						New Posts
					</span>
					<span class="flex items-center gap-1.5">
						<span class="inline-block w-3 h-3 rounded-sm bg-accent-green"></span>
						New Users
					</span>
				</div>
				<div class="flex items-end gap-px h-24 overflow-x-auto">
					{#each overview.growth as g}
						<div class="flex flex-col min-w-3 flex-1 h-full justify-end group relative">
							<!-- Tooltip -->
							<div
								class="absolute bottom-full mb-1 hidden group-hover:block bg-dark text-white text-sm rounded px-2 py-1 whitespace-nowrap z-10 pointer-events-none left-1/2 -translate-x-1/2"
							>
								{g.date}: {g.newPosts} posts · {g.newUsers} users
							</div>
							<div class="w-full flex gap-px items-end h-full">
								<div
									class="flex-1 bg-accent-blue rounded-t-sm"
									style="height:{g.newPosts
										? Math.max(Math.round((g.newPosts / maxVal) * 100), 2)
										: 0}%"
								></div>
								<div
									class="flex-1 bg-accent-green rounded-t-sm"
									style="height:{g.newUsers
										? Math.max(Math.round((g.newUsers / maxVal) * 100), 2)
										: 0}%"
								></div>
							</div>
						</div>
					{/each}
				</div>
			</DashCard>
		{/if}

		{#if data.role === 'admin'}
			<DashCard title="Visitor Countries">
				{#snippet actions()}
					<div class="flex gap-4 text-sm text-dark/60">
						<span>
							<strong class="text-dark">{visitorTotal}</strong>
							visits
						</span>
						<span>
							<strong class="text-dark">{visitorUnknown}</strong>
							unknown
						</span>
					</div>
				{/snippet}
				<p class="text-sm text-dark/50 -mt-2">Last 30 days, aggregated by Cloudflare country</p>
				{#if topVisitorCountries.length}
					<div class="grid sm:grid-cols-2 lg:grid-cols-4 gap-2">
						{#each topVisitorCountries as country}
							{@const pct = visitorTotal
								? Math.max(Math.round((country.visits / visitorTotal) * 100), 1)
								: 0}
							<div class="rounded-lg bg-background/40 p-3 flex flex-col gap-2">
								<div class="flex items-center justify-between gap-2">
									<span class="font-semibold truncate">{countryLabel(country.country_code)}</span>
									<span class="text-xs rounded-full bg-white px-2 py-0.5 text-dark/60">
										{country.country_code}
									</span>
								</div>
								<div class="h-2 rounded-full bg-white overflow-hidden">
									<div class="h-full rounded-full bg-primary" style="width:{pct}%"></div>
								</div>
								<span class="text-sm text-dark/60">{country.visits} visits</span>
							</div>
						{/each}
					</div>
				{:else}
					<EmptyState
						message="No visitor country data yet"
						hint="Data appears after requests include CF-IPCountry."
					/>
				{/if}
			</DashCard>
		{/if}

		<!-- ── Recent posts + Recent users ────────────── -->
		<div class="grid xl:grid-cols-2 gap-4">
			<!-- Recent Posts -->
			<DashCard title="Recent Posts">
				{#snippet actions()}
					<a href="/dashboard/posts" class="text-base text-primary hover:underline">View all →</a>
				{/snippet}
				{#if overview.recentPosts?.length}
					<ul class="flex flex-col">
						{#each overview.recentPosts as post}
							<li class="flex items-center gap-3 py-2 first:pt-0 last:pb-0">
								{#if post.coverUrl}
									<img
										src={post.coverUrl}
										alt=""
										class="w-10 h-10 rounded-lg object-cover shrink-0"
									/>
								{:else}
									<div class="w-10 h-10 rounded-lg bg-background/60 shrink-0"></div>
								{/if}
								<div class="flex-1 min-w-0">
									<a
										href="/dashboard/posts"
										class="text-base font-medium truncate block hover:text-primary"
									>
										{post.title}
									</a>
									<div class="flex items-center gap-2 text-sm text-dark/50">
										<span>by {post.authorName}</span>
										<span
											class="px-1.5 py-0.5 rounded-full {post.status === 'published'
												? 'bg-accent-green/20 text-accent-green'
												: 'bg-accent-yellow/30 text-dark/70'}"
										>
											{post.status}
										</span>
									</div>
								</div>
							</li>
						{/each}
					</ul>
				{:else}
					<EmptyState message="No posts yet" />
				{/if}
			</DashCard>

			<!-- Recent Users -->
			<DashCard title="Recent Registrations">
				{#snippet actions()}
					<a href="/dashboard/users" class="text-base text-primary hover:underline">View all →</a>
				{/snippet}
				{#if overview.recentUsers?.length}
					<ul class="flex flex-col">
						{#each overview.recentUsers as u}
							<li class="flex items-center gap-3 py-2 first:pt-0 last:pb-0">
								{#if u.avatarUrl}
									<img
										src={u.avatarUrl}
										alt=""
										class="w-8 h-8 rounded-full object-cover shrink-0"
									/>
								{:else}
									<div
										class="w-8 h-8 rounded-full bg-primary/20 flex items-center justify-center text-sm font-bold text-primary shrink-0"
									>
										{u.displayName.charAt(0).toUpperCase()}
									</div>
								{/if}
								<div class="flex-1 min-w-0">
									<p class="text-base font-medium truncate">{u.displayName}</p>
									<p class="text-sm text-dark/50">@{u.username}</p>
								</div>
								<span
									class="text-sm px-2 py-0.5 rounded-full shrink-0 {u.role === 'admin'
										? 'bg-accent-red/20 text-accent-red'
										: u.role === 'moderator'
											? 'bg-accent-blue/20 text-accent-blue'
											: 'bg-background/60 text-dark/60'}"
								>
									{roleLabel(u.role)}
								</span>
							</li>
						{/each}
					</ul>
				{:else}
					<EmptyState message="No users yet" />
				{/if}
			</DashCard>
		</div>
	{:else}
		<div class="rounded-xl bg-white">
			<EmptyState message="No data available." />
		</div>
	{/if}
</section>
