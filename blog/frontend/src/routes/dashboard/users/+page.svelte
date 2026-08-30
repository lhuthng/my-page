<script>
	import { onMount, untrack } from 'svelte';
	import { gql, fixUrl } from '$lib/api/graphql';
	import { authState } from '$lib/auth/user.svelte.js';
	import PageHeader from '$lib/components/dashboard/PageHeader.svelte';
	import SearchInput from '$lib/components/dashboard/SearchInput.svelte';
	import EmptyState from '$lib/components/dashboard/EmptyState.svelte';
	import LoadingCards from '$lib/components/dashboard/LoadingCards.svelte';
	import Heart from '$lib/components/svgs/Heart.svelte';
	import Diamond from '$lib/components/svgs/Diamond.svelte';
	import Club from '$lib/components/svgs/Club.svelte';
	import { fly } from 'svelte/transition';

	let { data } = $props();

	let userData = $state({
		users: untrack(() => data).users ?? [],
		total: untrack(() => data).total ?? 0,
		role_counts: untrack(() => data).role_counts ?? { admin: 0, moderator: 0, user: 0 }
	});
	let loading = $state(!untrack(() => data).users?.length);
	let loadingMore = $state(false);
	let error = $state(null);
	let search = $state('');
	let roleFilter = $state('');
	let debounceTimer;
	const LIMIT = 20;

	onMount(() => {
		if (!data.users?.length) fetchUsers(true);
	});

	let isAdmin = $derived(authState.user?.role === 'admin');

	function mapUser(item) {
		return {
			id: item.id,
			username: item.username,
			email: item.email,
			role: item.role,
			display_name: item.displayName,
			avatar_url: fixUrl(item.avatarUrl),
			created_at: item.createdAt
		};
	}

	async function fetchUsers(reset = false) {
		if (reset) {
			userData = {
				users: [],
				total: 0,
				role_counts: { admin: 0, moderator: 0, user: 0 }
			};
			loading = true;
		} else {
			if (loadingMore) return;
			loadingMore = true;
		}
		error = null;

		const variables = {
			limit: LIMIT,
			offset: reset ? 0 : userData.users.length
		};
		if (search.trim()) variables.search = search.trim();
		if (roleFilter) variables.role = roleFilter;

		try {
			const [usersResult, roleResult] = await Promise.all([
				gql.users(variables),
				reset ? gql.roleCounts() : null
			]);

			const items = usersResult.users.items.map(mapUser);
			userData = {
				users: reset ? items : [...userData.users, ...items],
				total: usersResult.users.total,
				role_counts: roleResult?.overview?.roleCounts ?? userData.role_counts
			};
		} catch (e) {
			error = e.message;
		} finally {
			loading = false;
			loadingMore = false;
		}
	}

	function onSearchInput() {
		clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => fetchUsers(true), 400);
	}

	let hasMore = $derived(userData.users.length < userData.total);

	function formatDate(str) {
		if (!str) return '-';
		return new Date(str.replace(' ', 'T')).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric'
		});
	}
</script>

<svelte:head>
	<title>Users - Dashboard | Huu Thang's Blog</title>
</svelte:head>

<section class="flex flex-col gap-4 *:bg-white *:rounded-xl *:p-4 pb-8">
	<!-- Role summary cards - override *: so individual cards style themselves -->
	<div class="grid grid-cols-3 gap-4 bg-transparent! p-0!">
		{#each [['Admins', userData.role_counts.admin, Heart, 'fill-accent-red', 'border-accent-red'], ['Moderators', userData.role_counts.moderator, Diamond, 'fill-accent-blue', 'border-accent-blue'], ['Users', userData.role_counts.user, Club, 'fill-dark/60', 'border-dark/30']] as [label, count, Icon, iconClass, borderClass]}
			<div class="bg-white rounded-xl p-4 flex items-center gap-3 border-l-4 {borderClass}">
				<Icon class="w-8 shrink-0 {iconClass}" />
				<div>
					<p class="text-2xl font-bold">{count}</p>
					<p class="text-base text-dark/60">{label}</p>
				</div>
			</div>
		{/each}
	</div>

	<!-- User list card -->
	<div class="bg-white rounded-xl p-4 flex flex-col gap-4">
		<PageHeader title="Users" count={userData.total} />

		<!-- Search + filter -->
		<div class="flex flex-wrap gap-2">
			<SearchInput
				placeholder="Search by name or username…"
				bind:value={search}
				onsearch={onSearchInput}
				onclear={() => fetchUsers(true)}
				class="flex-1 min-w-48"
			/>
			{#if isAdmin}
				<select
					bind:value={roleFilter}
					onchange={() => fetchUsers(true)}
					class="text-base bg-background/40 border-2 border-dark/10 focus:border-dark rounded-xl px-3 py-2 text-dark outline-none cursor-pointer"
				>
					<option value="">All Roles</option>
					<option value="admin">Admins</option>
					<option value="moderator">Moderators</option>
					<option value="user">Users</option>
				</select>
			{/if}
		</div>

		<!-- Content -->
		{#if loading}
			<LoadingCards count={5} tall />
		{:else if error}
			<p class="text-accent-red text-base">Error: {error}</p>
		{:else if userData.users.length === 0}
			<EmptyState
				message={search || roleFilter ? 'No users match your search' : 'No users found'}
				hint={search || roleFilter ? 'Try a different name or role filter.' : ''}
			/>
		{:else}
			<ul class="flex flex-col divide-y divide-background">
				{#each userData.users as u, i (u.username)}
					<li
						class="flex items-center gap-3 py-3 first:pt-0 last:pb-0 hover:bg-background/20 transition-colors rounded-lg px-2 -mx-2"
						in:fly={{ y: -10, duration: 200, delay: i * 15 }}
					>
						<!-- Avatar -->
						{#if u.avatar_url}
							<img src={u.avatar_url} alt="" class="w-10 h-10 rounded-full object-cover shrink-0" />
						{:else}
							<div
								class="w-10 h-10 rounded-full bg-primary/20 flex items-center justify-center font-bold text-primary shrink-0"
							>
								{u.display_name.charAt(0).toUpperCase()}
							</div>
						{/if}

						<!-- Name + username -->
						<div class="flex-1 min-w-0">
							<a
								href="/profiles/{u.username}"
								class="font-medium text-base truncate block hover:text-primary"
							>
								{u.display_name}
							</a>
							<p class="text-base text-dark/50">@{u.username}</p>
						</div>

						<!-- Role icon + label -->
						<span class="flex items-center gap-1 text-base shrink-0">
							{#if u.role === 'admin'}
								<Heart class="w-4 fill-accent-red" />
								<span class="text-accent-red font-medium">Admin</span>
							{:else if u.role === 'moderator'}
								<Diamond class="w-4 fill-accent-blue" />
								<span class="text-accent-blue font-medium">Moderator</span>
							{:else}
								<Club class="w-4 fill-dark/60" />
								<span class="text-dark/60">User</span>
							{/if}
						</span>

						<!-- Join date (hidden on small screens) -->
						<span class="hidden md:block text-base text-dark/40 shrink-0">
							{formatDate(u.created_at)}
						</span>
					</li>
				{/each}
			</ul>

			{#if hasMore}
				<div class="flex justify-center">
					<div class="duo-btn" data-duo-color="green">
						<button onclick={() => fetchUsers(false)} disabled={loadingMore}>
							{loadingMore
								? 'Loading…'
								: `Load more (${userData.total - userData.users.length} remaining)`}
						</button>
					</div>
				</div>
			{/if}
		{/if}
	</div>
</section>
