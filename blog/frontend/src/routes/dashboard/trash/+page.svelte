<script>
	import { api } from '$lib/api/client.js';

	let { data } = $props();
	let items = $state(data.items ?? []);
	let busyId = $state(null);
	let error = $state(data.error ?? null);

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

	async function purge(item) {
		if (!confirm(`Permanently delete "${item.title}" now? This cannot be undone.`)) return;
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
		}
	}
</script>

<svelte:head>
	<title>Trash - Dashboard</title>
</svelte:head>

<section class="bg-white rounded-xl p-4 flex flex-col gap-4">
	<h1 class="text-2xl font-semibold">Trash <span class="text-dark/40 text-lg font-normal">({items.length})</span></h1>
	<p class="text-sm text-dark/60">Items stay here for 7 days and can be restored. After that they are permanently deleted. Deleting a game keeps delegating projects but shows “Game unavailable”.</p>
	{#if error}<p class="text-sm text-accent-red">{error}</p>{/if}
	{#if items.length === 0}
		<p class="py-8 text-center text-dark/40">Trash is empty.</p>
	{:else}
		<ul class="flex flex-col gap-2">
			{#each items as item (item.post_id)}
				<li class="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-background p-3">
					<div class="flex flex-col gap-1">
						<span class="font-medium">{item.title} <span class="text-xs text-dark/40">/{item.slug}</span> <span class="ml-2 rounded bg-dark/10 px-2 py-0.5 text-xs">{item.content_kind}</span></span>
						<span class="text-xs text-dark/50">{item.deletion_reason ?? 'user_request'} · {daysLeft(item.scheduled_purge_at)} · deleted {item.deleted_at}</span>
					</div>
					<div class="flex gap-2">
						<button
							disabled={busyId === item.post_id}
							onclick={() => restore(item)}
							class="rounded-full border border-dark/20 px-4 py-1.5 text-sm hover:bg-dark/5 disabled:opacity-50"
						>
							Restore
						</button>
						<button
							disabled={busyId === item.post_id}
							onclick={() => purge(item)}
							class="rounded-full bg-accent-red px-4 py-1.5 text-sm font-medium text-white hover:bg-accent-red/90 disabled:opacity-50"
						>
							Delete now
						</button>
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</section>
