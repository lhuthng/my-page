<script>
	import { untrack } from 'svelte';
	import { auth } from '$lib/auth/user.svelte.js';

	let { data } = $props();

	let subscribers = $state(untrack(() => data).subscribers ?? []);
	let campaigns = $state(untrack(() => data).campaigns ?? []);
	let statusFilter = $state('');

	let filteredSubscribers = $derived(
		statusFilter ? subscribers.filter((s) => s.status === statusFilter) : subscribers
	);

	let subject = $state('');
	let bodyHtml = $state('');
	let bodyText = $state('');
	let postId = $state('');
	let sending = $state(false);
	let sendStatus = $state(true);
	let sendMessage = $state('');

	function formatDate(str) {
		if (!str) return '-';
		return new Date(str.replace(' ', 'T')).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric'
		});
	}

	async function handleSend(e) {
		e.preventDefault();
		sendMessage = '';

		if (!subject.trim() || !bodyHtml.trim()) {
			sendStatus = false;
			sendMessage = 'Subject and HTML body are required.';
			return;
		}

		sending = true;
		try {
			const payload = {
				subject: subject.trim(),
				body_html: bodyHtml
			};
			if (bodyText.trim()) payload.body_text = bodyText.trim();
			if (postId.trim()) payload.post_id = Number(postId.trim());

			const res = await fetch('/api/dashboard/newsletter/send', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					Authorization: auth()
				},
				body: JSON.stringify(payload)
			});

			let responsePayload;
			try {
				responsePayload = await res.json();
			} catch {
				responsePayload = { message: await res.text() };
			}

			sendStatus = res.ok;
			sendMessage = responsePayload.message ?? (res.ok ? 'Campaign sent.' : 'Send failed.');

			if (res.ok) {
				subject = '';
				bodyHtml = '';
				bodyText = '';
				postId = '';
				// Refresh campaign history
				try {
					const campaignsRes = await fetch('/api/dashboard/newsletter/campaigns', {
						headers: { Authorization: auth() }
					});
					if (campaignsRes.ok) {
						campaigns = (await campaignsRes.json()).campaigns ?? [];
					}
				} catch {
					/* non-fatal */
				}
			}
		} catch {
			sendStatus = false;
			sendMessage = 'Something went wrong. Please try again.';
		} finally {
			sending = false;
		}
	}
</script>

<svelte:head>
	<title>Newsletter - Dashboard | Huu Thang's Blog</title>
</svelte:head>

<section class="flex flex-col gap-4 *:bg-white *:rounded-xl *:p-4 pb-8">
	<!-- Subscribers -->
	<div class="flex flex-col gap-4">
		<div class="flex flex-wrap items-center justify-between gap-3">
			<h1 class="text-2xl font-semibold">
				Subscribers
				<span class="text-dark/40 text-lg font-normal">({filteredSubscribers.length})</span>
			</h1>
			<select
				bind:value={statusFilter}
				class="text-base bg-background/40 border border-background rounded-xl px-3 py-2 text-dark outline-none cursor-pointer"
			>
				<option value="">All Statuses</option>
				<option value="pending">Pending</option>
				<option value="confirmed">Confirmed</option>
				<option value="unsubscribed">Unsubscribed</option>
			</select>
		</div>

		{#if filteredSubscribers.length === 0}
			<div class="flex flex-col items-center gap-2 py-12 text-dark/40">
				<p class="text-lg">No subscribers found</p>
			</div>
		{:else}
			<div class="overflow-x-auto">
				<table class="w-full text-left text-base">
					<thead>
						<tr class="text-dark/50 border-b border-background">
							<th class="py-2 pr-4 font-medium">Email</th>
							<th class="py-2 pr-4 font-medium">Status</th>
							<th class="py-2 pr-4 font-medium">Subscribed</th>
							<th class="py-2 pr-4 font-medium">Confirmed</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-background">
						{#each filteredSubscribers as s (s.id)}
							<tr class="hover:bg-background/20 transition-colors">
								<td class="py-2 pr-4">{s.email}</td>
								<td class="py-2 pr-4">
									<span
										class="text-sm font-medium px-2 py-0.5 rounded-full text-dark/60"
										class:bg-accent-green-light-2={s.status === 'confirmed'}
										class:text-accent-green={s.status === 'confirmed'}
										class:bg-accent-yellow-light-2={s.status === 'pending'}
										class:text-accent-yellow={s.status === 'pending'}
										class:bg-background={s.status !== 'confirmed' && s.status !== 'pending'}
									>
										{s.status}
									</span>
								</td>
								<td class="py-2 pr-4 text-dark/60">{formatDate(s.created_at)}</td>
								<td class="py-2 pr-4 text-dark/60">{formatDate(s.confirmed_at)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</div>

	<!-- Send history -->
	<div class="flex flex-col gap-4">
		<h2 class="text-2xl font-semibold">
			Send History
			<span class="text-dark/40 text-lg font-normal">({campaigns.length})</span>
		</h2>

		{#if campaigns.length === 0}
			<div class="flex flex-col items-center gap-2 py-12 text-dark/40">
				<p class="text-lg">No campaigns sent yet</p>
			</div>
		{:else}
			<div class="overflow-x-auto">
				<table class="w-full text-left text-base">
					<thead>
						<tr class="text-dark/50 border-b border-background">
							<th class="py-2 pr-4 font-medium">Subject</th>
							<th class="py-2 pr-4 font-medium">Recipients</th>
							<th class="py-2 pr-4 font-medium">Success</th>
							<th class="py-2 pr-4 font-medium">Failed</th>
							<th class="py-2 pr-4 font-medium">Sent</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-background">
						{#each campaigns as c (c.id)}
							<tr class="hover:bg-background/20 transition-colors">
								<td class="py-2 pr-4">{c.subject}</td>
								<td class="py-2 pr-4 text-dark/60">{c.recipient_count}</td>
								<td class="py-2 pr-4 text-accent-green">{c.success_count}</td>
								<td class="py-2 pr-4 text-accent-red">{c.failure_count}</td>
								<td class="py-2 pr-4 text-dark/60">{formatDate(c.started_at)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</div>

	<!-- Compose -->
	<div class="flex flex-col gap-4">
		<h2 class="text-2xl font-semibold">Compose Campaign</h2>
		<form class="flex flex-col gap-3" onsubmit={handleSend}>
			<div class="flex flex-col gap-1">
				<label class="text-base text-dark/60" for="nl-subject">Subject</label>
				<input
					id="nl-subject"
					type="text"
					bind:value={subject}
					disabled={sending}
					class="bg-background/40 border border-background rounded-xl px-3 py-2 outline-none disabled:opacity-40"
				/>
			</div>
			<div class="flex flex-col gap-1">
				<label class="text-base text-dark/60" for="nl-post-id">Related Post ID (optional)</label>
				<input
					id="nl-post-id"
					type="number"
					bind:value={postId}
					disabled={sending}
					class="bg-background/40 border border-background rounded-xl px-3 py-2 outline-none disabled:opacity-40 sm:w-48"
				/>
			</div>
			<div class="flex flex-col gap-1">
				<label class="text-base text-dark/60" for="nl-body-html">HTML Body</label>
				<textarea
					id="nl-body-html"
					rows="8"
					bind:value={bodyHtml}
					disabled={sending}
					class="bg-background/40 border border-background rounded-xl px-3 py-2 outline-none disabled:opacity-40 font-mono text-sm"></textarea>
			</div>
			<div class="flex flex-col gap-1">
				<label class="text-base text-dark/60" for="nl-body-text">
					Plain Text Body (optional override)
				</label>
				<textarea
					id="nl-body-text"
					rows="4"
					bind:value={bodyText}
					disabled={sending}
					class="bg-background/40 border border-background rounded-xl px-3 py-2 outline-none disabled:opacity-40 font-mono text-sm"></textarea>
			</div>
			{#if sendMessage}
				<p class:text-accent-green={sendStatus} class:text-accent-red={!sendStatus}>
					*{sendMessage}
				</p>
			{/if}
			<div class="w-fit duo-btn" data-duo-color="green">
				<button type="submit" disabled={sending}>
					{sending ? 'Sending...' : 'Send Campaign'}
				</button>
			</div>
		</form>
	</div>
</section>
