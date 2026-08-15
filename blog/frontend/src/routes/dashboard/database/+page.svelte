<script>
	import { onMount, untrack } from 'svelte';
	import { page } from '$app/stores';
	import { gql, fixUrl } from '$lib/api/graphql';
	import { ApiError, api } from '$lib/api/client';

	let { data } = $props();

	const LIMIT = 20;

	let table = $state(untrack(() => data).table || 'users');
	let tableData = $state(null);
	let dbStats = $state(null);
	let currentPage = $state(1);
	let searchInput = $state('');
	let statusFilter = $state('');
	let roleFilter = $state('');
	let includeDeleted = $state(false);
	let loading = $state(true);
	let savingTagId = $state(null);
	let deletingTagId = $state(null);
	let editingTagId = $state(null);
	let editingTagName = $state('');
	let editingTagSlug = $state('');
	let editingTagDescription = $state('');
	let tagError = $state('');
	let tagToDelete = $state(null);

	const TABLES = [
		{ id: 'users', label: 'Users', statKey: 'totalUsers' },
		{ id: 'posts', label: 'Posts', statKey: 'totalPosts' },
		{ id: 'comments', label: 'Comments', statKey: 'totalComments' },
		{ id: 'media', label: 'Media', statKey: 'totalMedia' },
		{ id: 'series', label: 'Series', statKey: 'totalSeries' },
		{ id: 'tags', label: 'Tags', statKey: 'totalTags' },
		{ id: 'categories', label: 'Categories', statKey: 'totalCategories' }
	];

	let totalPages = $derived(Math.ceil((tableData?.total ?? 0) / LIMIT));

	const QUERIES = {
		users: `query Users($limit: Int, $offset: Int, $search: String, $role: String) { users(limit: $limit, offset: $offset, search: $search, role: $role) { total items { id username email role displayName bio avatarUrl createdAt } } }`,
		posts: `query Posts($limit: Int, $offset: Int, $search: String, $status: String) { posts(limit: $limit, offset: $offset, search: $search, status: $status) { total items { id title slug status authorName authorSlug seriesTitle viewCount isFeatured publishedAt createdAt updatedAt excerpt } } }`,
		comments: `query Comments($limit: Int, $offset: Int, $includeDeleted: Boolean) { comments(limit: $limit, offset: $offset, includeDeleted: $includeDeleted) { total items { id content postTitle postSlug authorName authorUsername parentId isDeleted createdAt } } }`,
		media: `query Media($limit: Int, $offset: Int, $search: String) { media(limit: $limit, offset: $offset, search: $search) { total items { id shortName fileName fileType url size description uploaderName useCount createdAt } } }`,
		series: `query Series($limit: Int, $offset: Int) { series(limit: $limit, offset: $offset) { total items { id title slug description postCount createdAt } } }`,
		tags: `query Tags($limit: Int, $offset: Int) { tags(limit: $limit, offset: $offset) { total items { id name slug description postCount } } }`,
		categories: `query Categories($limit: Int, $offset: Int) { categories(limit: $limit, offset: $offset) { total items { id name slug description postCount } } }`
	};

	const DB_STATS_QUERY = `query DbStats { dbStats { totalUsers totalPosts totalComments totalMedia totalSeries totalTags totalCategories } }`;

	function buildVariables() {
		const v = { limit: LIMIT, offset: (currentPage - 1) * LIMIT };
		if (table === 'users') {
			if (searchInput) v.search = searchInput;
			if (roleFilter) v.role = roleFilter;
		} else if (table === 'posts') {
			if (searchInput) v.search = searchInput;
			if (statusFilter) v.status = statusFilter;
		} else if (table === 'comments') v.includeDeleted = includeDeleted;
		else if (table === 'media') {
			if (searchInput) v.search = searchInput;
		}
		return v;
	}

	async function fetchData() {
		loading = true;
		try {
			const [statsResult, tableResult] = await Promise.all([
				gql.request(DB_STATS_QUERY),
				gql.request(QUERIES[table], buildVariables())
			]);
			dbStats = statsResult.dbStats;
			const raw = tableResult[table] ?? { items: [], total: 0 };
			if (raw.items) {
				raw.items = raw.items.map((r) => {
					if (r.avatarUrl) r.avatarUrl = fixUrl(r.avatarUrl);
					if (r.url) r.url = fixUrl(r.url);
					return r;
				});
			}
			tableData = raw;
		} catch {
			dbStats = null;
			tableData = { items: [], total: 0 };
		} finally {
			loading = false;
		}
	}

	onMount(() => fetchData());

	function navigate(params = {}) {
		if (params.table !== undefined) table = params.table;
		if (params.page !== undefined) currentPage = params.page;
		if (params.search !== undefined) searchInput = params.search;
		if (params.status !== undefined) statusFilter = params.status;
		if (params.role !== undefined) roleFilter = params.role;
		if (params.includeDeleted !== undefined) includeDeleted = params.includeDeleted;
		fetchData();
	}

	function switchTable(id) {
		searchInput = '';
		statusFilter = '';
		roleFilter = '';
		includeDeleted = false;
		table = id;
		currentPage = 1;
		fetchData();
	}

	function formatBytes(bytes) {
		if (!bytes) return '0 B';
		if (bytes < 1024) return bytes + ' B';
		if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
		return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
	}

	function fmtDate(iso) {
		return iso ? iso.slice(0, 10) : '-';
	}

	const isAdmin = $derived($page.data.role === 'admin');

	function startTagEdit(row) {
		editingTagId = row.id;
		editingTagName = row.name ?? '';
		editingTagSlug = row.slug ?? '';
		editingTagDescription = row.description ?? '';
		tagError = '';
	}

	function cancelTagEdit() {
		editingTagId = null;
		editingTagName = '';
		editingTagSlug = '';
		editingTagDescription = '';
		tagError = '';
	}

	function promptDeleteTag(row) {
		if (row.postCount > 0) return;
		tagError = '';
		tagToDelete = row;
	}

	function closeDeletePrompt() {
		tagToDelete = null;
	}

	async function saveTag(row) {
		savingTagId = row.id;
		tagError = '';
		try {
			const updated = await api.patch(`dashboard/tags/${row.id}`, {
				body: {
					name: editingTagName,
					slug: editingTagSlug,
					description: editingTagDescription
				}
			});

			tableData = {
				...tableData,
				items: tableData.items.map((item) => (item.id === row.id ? { ...item, ...updated } : item))
			};

			cancelTagEdit();
		} catch (error) {
			tagError = error instanceof ApiError ? error.message : 'Failed to update tag.';
		} finally {
			savingTagId = null;
		}
	}

	async function deleteTag(row) {
		if (row.postCount > 0) return;

		deletingTagId = row.id;
		tagError = '';
		try {
			await api.delete(`dashboard/tags/${row.id}`);
			tableData = {
				...tableData,
				total: Math.max(0, (tableData?.total ?? 1) - 1),
				items: tableData.items.filter((item) => item.id !== row.id)
			};
			if (editingTagId === row.id) {
				cancelTagEdit();
			}
			closeDeletePrompt();
		} catch (error) {
			tagError = error instanceof ApiError ? error.message : 'Failed to delete tag.';
		} finally {
			deletingTagId = null;
		}
	}
</script>

<svelte:head>
	<title>Database | Dashboard</title>
</svelte:head>

<div class="flex gap-4 items-start">
	<aside
		class="w-48 shrink-0 bg-white rounded-xl p-2 flex flex-col gap-0.5 self-start sticky top-4"
	>
		<p class="text-xs font-semibold text-dark/40 uppercase tracking-wider px-3 py-1.5">Tables</p>
		{#each TABLES as t}
			{@const count = dbStats?.[t.statKey] ?? 0}
			<button
				onclick={() => switchTable(t.id)}
				class="flex items-center justify-between px-3 py-2 rounded-lg text-sm font-medium transition-colors text-left w-full {table ===
				t.id
					? 'bg-dark text-white'
					: 'text-dark hover:bg-background/60'}"
			>
				<span>{t.label}</span>
				<span class="text-xs tabular-nums {table === t.id ? 'text-white/60' : 'text-dark/40'}">
					{count}
				</span>
			</button>
		{/each}
	</aside>

	<div class="flex-1 flex flex-col gap-3 min-w-0">
		<div class="bg-white rounded-xl p-4 flex flex-wrap items-center gap-3">
			<h2 class="text-xl font-semibold capitalize mr-auto">{table}</h2>
			{#if ['users', 'posts', 'media'].includes(table)}
				<form
					onsubmit={(e) => {
						e.preventDefault();
						navigate({ page: 1 });
					}}
					class="flex gap-2"
				>
					<input
						bind:value={searchInput}
						placeholder="Search…"
						class="border border-background rounded-lg px-3 py-1.5 text-sm outline-none focus:border-dark/30 w-44 transition-colors"
					/>
					<button
						type="submit"
						class="px-3 py-1.5 bg-dark text-white rounded-lg text-sm hover:bg-dark/80 transition-colors"
					>
						Search
					</button>
				</form>
			{/if}
			{#if table === 'posts'}
				<select
					bind:value={statusFilter}
					onchange={() => navigate({ page: 1 })}
					class="border border-background rounded-lg px-3 py-1.5 text-sm outline-none bg-white cursor-pointer"
				>
					<option value="">All Status</option>
					<option value="published">Published</option>
					<option value="draft">Draft</option>
					<option value="archived">Archived</option>
				</select>
			{/if}
			{#if table === 'users'}
				<select
					bind:value={roleFilter}
					onchange={() => navigate({ page: 1 })}
					class="border border-background rounded-lg px-3 py-1.5 text-sm outline-none bg-white cursor-pointer"
				>
					<option value="">All Roles</option>
					<option value="admin">Admin</option>
					<option value="moderator">Moderator</option>
					<option value="user">User</option>
				</select>
			{/if}
			{#if table === 'comments'}
				<label class="flex items-center gap-2 text-sm cursor-pointer select-none">
					<input
						type="checkbox"
						bind:checked={includeDeleted}
						onchange={() => navigate({ page: 1 })}
						class="accent-primary w-4 h-4"
					/>
					Show deleted
				</label>
			{/if}
		</div>

		<div class="bg-white rounded-xl overflow-hidden">
			<div
				class="px-4 py-2.5 border-b border-background/60 flex items-center gap-3 text-sm text-dark/50"
			>
				<span>{tableData?.total ?? 0} records</span>
				{#if loading}
					<span class="animate-pulse text-primary font-medium">Loading…</span>
				{/if}
			</div>

			<div class="overflow-x-auto">
				<table class="w-full text-sm">
					<thead class="bg-background/40">
						<tr>
							{#if table === 'users'}
								{#each ['#', 'Username', 'Display Name', 'Email', 'Role', 'Created'] as h}
									<th class="px-4 py-2.5 text-left font-semibold text-dark/50 whitespace-nowrap">
										{h}
									</th>
								{/each}
							{:else if table === 'posts'}
								{#each ['#', 'Title', 'Author', 'Status', 'Series', 'Views', 'Featured', 'Created'] as h}
									<th class="px-4 py-2.5 text-left font-semibold text-dark/50 whitespace-nowrap">
										{h}
									</th>
								{/each}
							{:else if table === 'comments'}
								{#each ['#', 'Content', 'Post', 'Author', 'Status', 'Created'] as h}
									<th class="px-4 py-2.5 text-left font-semibold text-dark/50 whitespace-nowrap">
										{h}
									</th>
								{/each}
							{:else if table === 'media'}
								{#each ['#', 'Short Name', 'File Name', 'Type', 'Size', 'Uses', 'Created'] as h}
									<th class="px-4 py-2.5 text-left font-semibold text-dark/50 whitespace-nowrap">
										{h}
									</th>
								{/each}
							{:else if table === 'series'}
								{#each ['#', 'Title', 'Slug', 'Posts', 'Created'] as h}
									<th class="px-4 py-2.5 text-left font-semibold text-dark/50 whitespace-nowrap">
										{h}
									</th>
								{/each}
							{:else if table === 'tags'}
								{#each ['#', 'Name', 'Slug', 'Description', 'Posts', 'Actions'] as h}
									<th class="px-4 py-2.5 text-left font-semibold text-dark/50 whitespace-nowrap">
										{h}
									</th>
								{/each}
							{:else if table === 'categories'}
								{#each ['#', 'Name', 'Slug', 'Description', 'Posts'] as h}
									<th class="px-4 py-2.5 text-left font-semibold text-dark/50 whitespace-nowrap">
										{h}
									</th>
								{/each}
							{/if}
						</tr>
					</thead>
					<tbody
						class="divide-y divide-background/60 transition-opacity duration-150 {loading
							? 'opacity-40'
							: 'opacity-100'}"
					>
						{#if tableData?.items?.length}
							{#each tableData.items as row, i}
								{@const rowNum = (currentPage - 1) * LIMIT + i + 1}
								<tr class="hover:bg-background/30 transition-colors">
									<td class="px-4 py-2.5 text-dark/35 tabular-nums text-xs">{rowNum}</td>
									{#if table === 'users'}
										<td class="px-4 py-2.5 font-medium whitespace-nowrap">@{row.username}</td>
										<td class="px-4 py-2.5">{row.displayName}</td>
										<td class="px-4 py-2.5 text-dark/60">{row.email}</td>
										<td class="px-4 py-2.5">
											<span
												class="px-2 py-0.5 rounded-full text-xs font-medium whitespace-nowrap {row.role ===
												'admin'
													? 'bg-accent-red/15 text-accent-red'
													: row.role === 'moderator'
														? 'bg-accent-blue/15 text-accent-blue'
														: 'bg-background/80 text-dark/60'}"
											>
												{row.role}
											</span>
										</td>
										<td class="px-4 py-2.5 text-dark/45 text-xs">{fmtDate(row.createdAt)}</td>
									{:else if table === 'posts'}
										<td class="px-4 py-2.5 font-medium max-w-[16rem] truncate" title={row.title}>
											{row.title}
										</td>
										<td class="px-4 py-2.5 text-dark/60 whitespace-nowrap">
											{row.authorName ?? '-'}
										</td>
										<td class="px-4 py-2.5">
											<span
												class="px-2 py-0.5 rounded-full text-xs font-medium whitespace-nowrap {row.status ===
												'published'
													? 'bg-accent-green/15 text-accent-green'
													: row.status === 'draft'
														? 'bg-accent-yellow/25 text-dark/70'
														: 'bg-background/80 text-dark/50'}"
											>
												{row.status}
											</span>
										</td>
										<td
											class="px-4 py-2.5 text-dark/45 text-xs max-w-40 truncate"
											title={row.seriesTitle ?? ''}
										>
											{row.seriesTitle ?? '-'}
										</td>
										<td class="px-4 py-2.5 text-dark/60 tabular-nums">{row.viewCount}</td>
										<td class="px-4 py-2.5 text-center">{row.isFeatured ? '⭐' : '-'}</td>
										<td class="px-4 py-2.5 text-dark/45 text-xs whitespace-nowrap">
											{fmtDate(row.createdAt)}
										</td>
									{:else if table === 'comments'}
										<td class="px-4 py-2.5 text-dark/80 max-w-[18rem] truncate" title={row.content}>
											{row.content}
										</td>
										<td
											class="px-4 py-2.5 text-dark/55 text-xs max-w-48 truncate"
											title={row.postTitle}
										>
											{row.postTitle}
										</td>
										<td class="px-4 py-2.5 text-dark/60 whitespace-nowrap">
											{row.authorName ?? 'Anonymous'}
										</td>
										<td class="px-4 py-2.5">
											{#if row.isDeleted}
												<span
													class="px-2 py-0.5 rounded-full text-xs font-medium bg-accent-red/15 text-accent-red"
												>
													Deleted
												</span>
											{:else}
												<span
													class="px-2 py-0.5 rounded-full text-xs font-medium bg-accent-green/15 text-accent-green"
												>
													Active
												</span>
											{/if}
										</td>
										<td class="px-4 py-2.5 text-dark/45 text-xs whitespace-nowrap">
											{fmtDate(row.createdAt)}
										</td>
									{:else if table === 'media'}
										<td class="px-4 py-2.5 font-mono text-xs text-dark/70 whitespace-nowrap">
											{row.shortName}
										</td>
										<td
											class="px-4 py-2.5 text-dark/70 max-w-[16rem] truncate text-xs"
											title={row.fileName}
										>
											{row.fileName}
										</td>
										<td class="px-4 py-2.5 text-dark/45 text-xs whitespace-nowrap">
											{row.fileType}
										</td>
										<td class="px-4 py-2.5 text-dark/60 tabular-nums text-xs whitespace-nowrap">
											{formatBytes(row.size)}
										</td>
										<td class="px-4 py-2.5 text-dark/60 tabular-nums">{row.useCount}</td>
										<td class="px-4 py-2.5 text-dark/45 text-xs whitespace-nowrap">
											{fmtDate(row.createdAt)}
										</td>
									{:else if table === 'series'}
										<td class="px-4 py-2.5 font-medium">{row.title}</td>
										<td class="px-4 py-2.5 font-mono text-xs text-dark/55">{row.slug}</td>
										<td class="px-4 py-2.5 text-dark/60 tabular-nums">{row.postCount}</td>
										<td class="px-4 py-2.5 text-dark/45 text-xs whitespace-nowrap">
											{fmtDate(row.createdAt)}
										</td>
									{:else if table === 'tags'}
										<td class="px-4 py-2.5">
											{#if isAdmin && editingTagId === row.id}
												<input
													bind:value={editingTagName}
													class="w-full min-w-40 rounded-lg border border-background px-3 py-2 text-sm text-dark/80 outline-none focus:border-dark/30"
												/>
											{:else}
												<span class="font-medium">{row.name}</span>
											{/if}
										</td>
										<td class="px-4 py-2.5">
											{#if isAdmin && editingTagId === row.id}
												<input
													bind:value={editingTagSlug}
													class="w-full min-w-44 rounded-lg border border-background px-3 py-2 font-mono text-xs text-dark/75 outline-none focus:border-dark/30"
												/>
											{:else}
												<span class="font-mono text-xs text-dark/55">{row.slug}</span>
											{/if}
										</td>
										<td class="px-4 py-2.5">
											{#if isAdmin && editingTagId === row.id}
												<div class="space-y-1">
													<textarea
														bind:value={editingTagDescription}
														rows="3"
														class="w-full min-w-56 rounded-lg border border-background px-3 py-2 text-xs text-dark/75 outline-none focus:border-dark/30"
														placeholder="Optional description"></textarea>
													{#if tagError}
														<p class="text-xs text-accent-red">{tagError}</p>
													{/if}
												</div>
											{:else}
												<div
													class="max-w-[18rem] truncate text-xs text-dark/45"
													title={row.description ?? ''}
												>
													{row.description ?? '-'}
												</div>
											{/if}
										</td>
										<td class="px-4 py-2.5 text-dark/60 tabular-nums">{row.postCount}</td>
										<td class="px-4 py-2.5">
											{#if isAdmin}
												{#if editingTagId === row.id}
													<div class="flex items-center gap-2">
														<button
															onclick={() => saveTag(row)}
															disabled={savingTagId === row.id}
															class="rounded-lg bg-dark px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-dark/85 disabled:opacity-50"
														>
															{savingTagId === row.id ? 'Saving...' : 'Save'}
														</button>
														<button
															onclick={cancelTagEdit}
															disabled={savingTagId === row.id || deletingTagId === row.id}
															class="rounded-lg border border-background px-3 py-1.5 text-sm text-dark/65 transition-colors hover:bg-background/60 disabled:opacity-50"
														>
															Cancel
														</button>
														<button
															onclick={() => promptDeleteTag(row)}
															disabled={row.postCount > 0 ||
																savingTagId === row.id ||
																deletingTagId === row.id}
															class="rounded-lg border border-accent-red/20 px-3 py-1.5 text-sm text-accent-red transition-colors hover:bg-accent-red/8 disabled:cursor-not-allowed disabled:opacity-40"
														>
															{deletingTagId === row.id ? 'Deleting...' : 'Delete'}
														</button>
													</div>
												{:else}
													<div class="flex items-center gap-2">
														<button
															onclick={() => startTagEdit(row)}
															class="rounded-lg border border-background px-3 py-1.5 text-sm text-dark/70 transition-colors hover:bg-background/60"
														>
															Edit
														</button>
														<button
															onclick={() => promptDeleteTag(row)}
															disabled={row.postCount > 0 || deletingTagId === row.id}
															class="rounded-lg border border-accent-red/20 px-3 py-1.5 text-sm text-accent-red transition-colors hover:bg-accent-red/8 disabled:cursor-not-allowed disabled:opacity-40"
															title={row.postCount > 0
																? 'Only unused tags can be deleted.'
																: 'Delete this unused tag'}
														>
															{deletingTagId === row.id ? 'Deleting...' : 'Delete'}
														</button>
													</div>
												{/if}
											{:else}
												<span class="text-xs text-dark/30">Admin only</span>
											{/if}
										</td>
									{:else if table === 'categories'}
										<td class="px-4 py-2.5 font-medium">{row.name}</td>
										<td class="px-4 py-2.5 font-mono text-xs text-dark/55">{row.slug}</td>
										<td
											class="px-4 py-2.5 text-dark/45 text-xs max-w-[16rem] truncate"
											title={row.description ?? ''}
										>
											{row.description ?? '-'}
										</td>
										<td class="px-4 py-2.5 text-dark/60 tabular-nums">{row.postCount}</td>
									{/if}
								</tr>
							{/each}
						{:else}
							<tr>
								<td colspan="10" class="px-4 py-12 text-center text-dark/35">
									{loading ? 'Loading…' : 'No records found'}
								</td>
							</tr>
						{/if}
					</tbody>
				</table>
			</div>

			{#if totalPages > 1}
				<div
					class="px-4 py-3 border-t border-background/60 flex items-center justify-between gap-4"
				>
					<button
						onclick={() => navigate({ page: currentPage - 1 })}
						disabled={currentPage <= 1 || loading}
						class="px-3 py-1.5 rounded-lg text-sm border border-background disabled:opacity-40 disabled:cursor-not-allowed hover:bg-background/60 transition-colors"
					>
						← Prev
					</button>
					<span class="text-sm text-dark/55 tabular-nums">Page {currentPage} of {totalPages}</span>
					<button
						onclick={() => navigate({ page: currentPage + 1 })}
						disabled={currentPage >= totalPages || loading}
						class="px-3 py-1.5 rounded-lg text-sm border border-background disabled:opacity-40 disabled:cursor-not-allowed hover:bg-background/60 transition-colors"
					>
						Next →
					</button>
				</div>
			{/if}
		</div>
	</div>
</div>

{#if tagToDelete}
	<div class="fixed inset-0 z-40 flex items-center justify-center bg-dark/45 px-4">
		<div
			class="w-full max-w-md rounded-2xl bg-white p-5 shadow-2xl"
			role="dialog"
			aria-modal="true"
			aria-labelledby="delete-tag-title"
		>
			<div class="space-y-3">
				<div>
					<h3 id="delete-tag-title" class="text-lg font-semibold text-dark">Delete unused tag?</h3>
					<p class="mt-1 text-sm text-dark/65">
						<span class="font-medium text-dark">{tagToDelete.name}</span>
						will be removed permanently. This is only available because the tag has no current usage.
					</p>
				</div>
				{#if tagError}
					<p class="rounded-lg bg-accent-red/10 px-3 py-2 text-sm text-accent-red">{tagError}</p>
				{/if}
				<div class="flex items-center justify-end gap-2">
					<button
						onclick={closeDeletePrompt}
						disabled={deletingTagId === tagToDelete.id}
						class="rounded-lg border border-background px-4 py-2 text-sm text-dark/70 transition-colors hover:bg-background/60 disabled:opacity-50"
					>
						Cancel
					</button>
					<button
						onclick={() => deleteTag(tagToDelete)}
						disabled={deletingTagId === tagToDelete.id}
						class="rounded-lg bg-accent-red px-4 py-2 text-sm font-medium text-white transition-colors hover:opacity-90 disabled:opacity-50"
					>
						{deletingTagId === tagToDelete.id ? 'Deleting...' : 'Delete tag'}
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}
