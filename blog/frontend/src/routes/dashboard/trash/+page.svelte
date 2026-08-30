<script>
	import { api } from '$lib/api/client.js';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import EmptyState from '$lib/components/dashboard/EmptyState.svelte';

	let { data } = $props();
	let items = $state(data.items ?? []);
	let busyId = $state(null);
	let error = $state(data.error ?? null);

	// Purge confirmation dialog state
	let purgeTarget = $state(null);
	let dialogOpen = $state(false);

	function daysLeft(scheduled) {
		if (!scheduled) return '';
		const end = new Date(scheduled.replace(' ', 'T') + 'Z');
		const diff = end - new Date();
		if (diff <= 0) return 'expires soon';
		const d = Math.floor(diff / 86400000);
		const h = Math.floor((diff % 86400000) / 3600000);
		return `${d}d ${h}h left`;
	}

	async function restore(item) {
		busyId = item.post_id;
		try {
			if (item.content_kind === 'project' && item.project_id) {
				await api.post(`projects/id/${item.project_id}/restore`);
			} else if (item.content_kind === 'game' && item.game_id) {
				await api.post(`games/id/${item.game_id}/restore`);
			} else {
				await api.post(`posts/id/${item.post_id}/restore`);
			}
			items = items.filter((i) => i.post_id !== item.post_id);
		} catch (e) {
			error = e.message;
		} finally {
			busyId = null;
		}
	}

	function askPurge(item) {
		purgeTarget = item;
		dialogOpen = true;
	}

	async function purge() {
		const item = purgeTarget;
		dialogOpen = false;
		if (!item) return;
		busyId = item.post_id;
		try {
			if (item.content_kind === 'project' && item.project_id) {
				await api.delete(`projects/id/${item.project_id}/purge`);
			} else if (item.content_kind === 'game' && item.game_id) {
				await api.delete(`games/id/${item.game_id}/purge`);
			} else {
				await api.delete(`posts/id/${item.post_id}/purge`);
			}
			items = items.filter((i) => i.post_id !== item.post_id);
		} catch (e) {
			error = e.message;
		} finally {
			busyId = null;
			purgeTarget = null;
		}
	}
</script>

<svelte:head>
	<title>Trash - Dashboard</title>
</svelte:head>

<section class="bg-white rounded-xl p-4 flex flex-col gap-4">
	<header class="flex flex-wrap items-center justify-between gap-2">
		<h1 class="text-2xl font-semibold">
			Trash <span class="text-dark/40 text-lg font-normal">({items.length})</span>
		</h1>
	</header>
	<p class="text-sm text-dark/60">
		Items stay here for 7 days and can be restored. After that they are permanently deleted.
		Deleting a game keeps delegating projects but shows "Game unavailable".
	</p>
	{#if error}<p class="text-sm text-accent-red">{error}</p>{/if}
	{#if items.length === 0}
		<EmptyState message="Trash is empty." hint="Deleted posts, projects, and games rest here for 7 days." mascot />
	{:else}
		<ul class="flex flex-col gap-2">
			{#each items as item (item.post_id)}
				<li
					class="flex flex-wrap items-center justify-between gap-3 rounded-xl border-2 border-dark/10 bg-background/30 p-3 hover:border-dark/20 transition-colors"
				>
					<div class="flex flex-col gap-1 min-w-0">
						<span class="font-medium flex items-center gap-2 flex-wrap">
							{item.title}
							<span class="text-xs text-dark/40 font-mono">/{item.slug}</span>
							<span
								class="rounded-full bg-primary/20 px-2 py-0.5 text-xs text-dark/60 uppercase"
							>
								{item.content_kind}
							</span>
						</span>
						<span class="text-xs text-dark/50">
							{item.deletion_reason ?? 'user_request'} · {daysLeft(item.scheduled_purge_at)} ·
							deleted {item.deleted_at}
						</span>
					</div>
					<div class="flex gap-2 shrink-0">
						<button
							disabled={busyId === item.post_id}
							onclick={() => restore(item)}
							class="rounded-full border-2 border-accent-green text-accent-green bg-accent-green-light-2/50 px-4 py-1.5 text-sm font-medium hover:bg-accent-green hover:text-white transition-colors disabled:opacity-50 cursor-pointer"
						>
							Restore
						</button>
						<button
							disabled={busyId === item.post_id}
							onclick={() => askPurge(item)}
							class="rounded-full border-2 border-accent-red text-accent-red bg-accent-red-light-2/50 px-4 py-1.5 text-sm font-medium hover:bg-accent-red hover:text-white transition-colors disabled:opacity-50 cursor-pointer"
						>
							Delete now
						</button>
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</section>

<ConfirmDialog
	open={dialogOpen}
	title="Delete permanently?"
	description={`"${purgeTarget?.title ?? ''}" will be removed for good. This cannot be undone.`}
	confirmLabel="Delete now"
	confirmColor="red"
	onconfirm={purge}
	oncancel={() => (dialogOpen = false)}
/>
