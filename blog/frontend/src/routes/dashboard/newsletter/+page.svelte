<script>
	import { untrack } from 'svelte';
	import { auth } from '$lib/auth/user.svelte.js';
	import PageHeader from '$lib/components/dashboard/PageHeader.svelte';
	import DashCard from '$lib/components/dashboard/DashCard.svelte';
	import EmptyState from '$lib/components/dashboard/EmptyState.svelte';

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

<section class="flex flex-col gap-4 pb-8">
	<!-- Subscribers -->
	<DashCard title="Subscribers" count={filteredSubscribers.length}>
		{#snippet actions()}
			<select
				bind:value={statusFilter}
				class="text-base bg-background/40 border-2 border-dark/10 focus:border-dark rounded-xl px-3 py-2 text-dark outline-none cursor-pointer"
			>
				<option value="">All Statuses</option>
				<option value="pending">Pending</option>
				<option value="confirmed">Confirmed</option>
				<option value="unsubscribed">Unsubscribed</option>
			</select>
		{/snippet}

		{#if filteredSubscribers.length === 0}
			<EmptyState message="No subscribers found" hint="Sign-ups from the footer form land here." />
		{:else}
			<div class="overflow-x-auto rounded-xl border-2 border-dark">
				<table class="w-full text-left text-sm">
					<thead class="bg-dark text-white">
						<tr>
							<th class="px-3 py-2 font-semibold">Email</th>
							<th class="px-3 py-2 font-semibold">Status</th>
							<th class="px-3 py-2 font-semibold">Subscribed</th>
							<th class="px-3 py-2 font-semibold">Confirmed</th>
						</tr>
					</thead>
					<tbody class="divide-y-2 divide-dark/10">
						{#each filteredSubscribers as s (s.id)}
							<tr class="hover:bg-background/40 transition-colors">
								<td class="px-3 py-2 font-medium">{s.email}</td>
								<td class="px-3 py-2">
									<span
										class="text-xs font-medium px-2 py-0.5 rounded-full text-dark/60"
										class:bg-accent-green-light-2={s.status === 'confirmed'}
										class:text-accent-green={s.status === 'confirmed'}
										class:bg-accent-yellow-light-2={s.status === 'pending'}
										class:text-accent-yellow={s.status === 'pending'}
										class:bg-background={s.status !== 'confirmed' && s.status !== 'pending'}
									>
										{s.status}
									</span>
								</td>
								<td class="px-3 py-2 text-dark/60">{formatDate(s.created_at)}</td>
								<td class="px-3 py-2 text-dark/60">{formatDate(s.confirmed_at)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</DashCard>

	<!-- Send history -->
	<DashCard title="Send History" count={campaigns.length}>
		{#if campaigns.length === 0}
			<EmptyState message="No campaigns sent yet" />
		{:else}
			<div class="overflow-x-auto rounded-xl border-2 border-dark">
				<table class="w-full text-left text-sm">
					<thead class="bg-dark text-white">
						<tr>
							<th class="px-3 py-2 font-semibold">Subject</th>
							<th class="px-3 py-2 font-semibold">Recipients</th>
							<th class="px-3 py-2 font-semibold">Success</th>
							<th class="px-3 py-2 font-semibold">Failed</th>
							<th class="px-3 py-2 font-semibold">Sent</th>
						</tr>
					</thead>
					<tbody class="divide-y-2 divide-dark/10">
						{#each campaigns as c (c.id)}
							<tr class="hover:bg-background/40 transition-colors">
								<td class="px-3 py-2 font-medium">{c.subject}</td>
								<td class="px-3 py-2 text-dark/60">{c.recipient_count}</td>
								<td class="px-3 py-2 text-accent-green">{c.success_count}</td>
								<td class="px-3 py-2 text-accent-red">{c.failure_count}</td>
								<td class="px-3 py-2 text-dark/60">{formatDate(c.started_at)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</DashCard>

	<!-- Compose -->
	<DashCard title="Compose Campaign">
		<form class="flex flex-col gap-3" onsubmit={handleSend}>
			<div class="flex flex-col gap-1">
				<label class="text-base text-dark/60" for="nl-subject">Subject</label>
				<input
					id="nl-subject"
					type="text"
					bind:value={subject}
					disabled={sending}
					class="bg-background/40 border-2 border-dark/10 focus:border-dark rounded-xl px-3 py-2 outline-none disabled:opacity-40"
				/>
			</div>
			<div class="flex flex-col gap-1">
				<label class="text-base text-dark/60" for="nl-post-id">Related Post ID (optional)</label>
				<input
					id="nl-post-id"
					type="number"
					bind:value={postId}
					disabled={sending}
					class="bg-background/40 border-2 border-dark/10 focus:border-dark rounded-xl px-3 py-2 outline-none disabled:opacity-40 sm:w-48"
				/>
			</div>
			<div class="flex flex-col gap-1">
				<label class="text-base text-dark/60" for="nl-body-html">HTML Body</label>
				<textarea
					id="nl-body-html"
					rows="8"
					bind:value={bodyHtml}
					disabled={sending}
					class="bg-background/40 border-2 border-dark/10 focus:border-dark rounded-xl px-3 py-2 outline-none disabled:opacity-40 font-mono text-sm custom-scrollbar"></textarea>
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
					class="bg-background/40 border-2 border-dark/10 focus:border-dark rounded-xl px-3 py-2 outline-none disabled:opacity-40 font-mono text-sm custom-scrollbar"></textarea>
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
	</DashCard>
</section>
