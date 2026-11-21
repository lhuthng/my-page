<script>
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client.js';
	import ConfirmDialog from '../ui/ConfirmDialog.svelte';

	/**
	 * The author's invitation panel for one game: the open/invite-only switch
	 * (saved with the game) and the codes themselves. The raw code exists only
	 * in the create response — the list carries a hint, never the secret — so
	 * a code shown here is copied once and stored wherever the author keeps
	 * it.
	 */
	let { gameId, accessMode = $bindable('open'), feedback } = $props();

	let invitations = $state([]);
	let loading = $state(true);
	let loadError = $state('');
	let newMaxUses = $state('1');
	let newNote = $state('');
	let creating = $state(false);
	// The code the server just minted; rendered once, with a copy button.
	let freshCode = $state(null);
	let freshCopied = $state(false);
	let revokeTarget = $state(null);
	let revoking = $state(false);

	const inputClasses =
		'w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white placeholder:text-dark/30 focus:placeholder:text-white/70';

	onMount(load);

	async function load() {
		loading = true;
		loadError = '';
		try {
			const data = await api.get(`games/id/${gameId}/invitations`);
			invitations = data.invitations ?? [];
		} catch (e) {
			loadError = e?.message ?? 'Could not load the invitation codes.';
		} finally {
			loading = false;
		}
	}

	function capacityLabel(maxUses) {
		if (maxUses === 0) return 'unlimited';
		if (maxUses === 1) return '1 use';
		return `${maxUses} uses`;
	}

	async function createCode() {
		if (creating) return;
		creating = true;
		freshCode = null;
		freshCopied = false;
		try {
			const data = await api.post(`games/id/${gameId}/invitations`, {
				body: {
					max_uses: Number(newMaxUses),
					note: newNote.trim() || null
				}
			});
			freshCode = data.code;
			invitations = [data.invitation, ...invitations];
			newNote = '';
			feedback?.toast('Code created — copy it now, it is shown only once.');
		} catch (e) {
			feedback?.toast(e?.message ?? 'Could not create a code.', { tone: 'error' });
		} finally {
			creating = false;
		}
	}

	async function copyFresh() {
		try {
			await navigator.clipboard.writeText(freshCode);
			freshCopied = true;
		} catch {
			feedback?.toast('Copy failed — select the code and copy by hand.', { tone: 'error' });
		}
	}

	async function revoke() {
		if (!revokeTarget || revoking) return;
		revoking = true;
		try {
			await api.delete(`games/id/${gameId}/invitations/${revokeTarget.id}`);
			invitations = invitations.map((invitation) =>
				invitation.id === revokeTarget.id ? { ...invitation, revoked: true } : invitation
			);
			feedback?.toast('Code revoked. Anyone holding it loses access.');
		} catch (e) {
			feedback?.toast(e?.message ?? 'Could not revoke the code.', { tone: 'error' });
		} finally {
			revoking = false;
			revokeTarget = null;
		}
	}
</script>

<section class="bg-white rounded-xl p-4">
	<div class="space-y-2">
		<div class="flex items-center gap-3 mb-3">
			<h2 class="text-xl lg:text-2xl">Invitations</h2>
			<hr class="grow border" />
		</div>
	</div>

	<div class="grid gap-4 lg:grid-cols-[minmax(0,14rem)_1fr]">
		<div class="space-y-3">
			<label class="flex flex-col gap-1 text-sm font-medium text-dark/60" for="access-mode">
				Who can play
				<select
					id="access-mode"
					bind:value={accessMode}
					class={`${inputClasses} disabled:opacity-60`}
				>
					<option value="open">Everyone — public</option>
					<option value="invited">Invite only — code required</option>
				</select>
			</label>
			<p class="text-sm text-dark/60">
				Invite-only blocks the game files server-side; a player without a code sees only an entry
				form. Save the game to apply.
			</p>
		</div>

		<div class="space-y-3">
			<div class="flex flex-col gap-2 sm:flex-row">
				<select bind:value={newMaxUses} class={`${inputClasses} sm:w-40`} aria-label="Uses allowed">
					<option value="1">Single use</option>
					<option value="0">Unlimited</option>
					<option value="5">5 uses</option>
					<option value="10">10 uses</option>
					<option value="25">25 uses</option>
				</select>
				<input
					class={inputClasses}
					placeholder="Note (who it is for)"
					bind:value={newNote}
					maxlength="120"
					aria-label="Code note"
				/>
				<div class="duo-btn shrink-0" data-duo-color="blue">
					<button onclick={createCode} disabled={creating}>
						{creating ? 'Creating…' : 'Create code'}
					</button>
				</div>
			</div>

			{#if freshCode}
				<div class="space-y-2 rounded-xl border-2 border-dark bg-primary/10 p-3">
					<p class="text-sm font-medium text-dark/70">
						Shown once — copy it before you leave this page:
					</p>
					<div class="flex items-center gap-2">
						<code class="grow font-mono text-lg font-bold tracking-wider text-dark break-all">
							{freshCode}
						</code>
						<div class="duo-btn shrink-0" data-duo-color="green">
							<button onclick={copyFresh}>{freshCopied ? 'Copied!' : 'Copy'}</button>
						</div>
					</div>
				</div>
			{/if}

			{#if loading}
				<p class="text-dark/60">Loading codes…</p>
			{:else if loadError}
				<p class="text-sm font-medium text-accent-red">{loadError}</p>
			{:else if invitations.length === 0}
				<p class="text-dark/60">No codes yet. Create one and hand it to a player.</p>
			{:else}
				<div class="overflow-x-auto">
					<table class="w-full text-left text-sm">
						<thead>
							<tr class="text-dark/60">
								<th class="py-1 pr-3 font-medium">Note</th>
								<th class="py-1 pr-3 font-medium">Code</th>
								<th class="py-1 pr-3 font-medium">Capacity</th>
								<th class="py-1 pr-3 font-medium">Used</th>
								<th class="py-1 pr-3 font-medium">Status</th>
								<th class="py-1 font-medium"><span class="sr-only">Actions</span></th>
							</tr>
						</thead>
						<tbody>
							{#each invitations as invitation (invitation.id)}
								<tr class="border-t border-dark/10">
									<td class="py-2 pr-3 text-dark">{invitation.note || '—'}</td>
									<td class="py-2 pr-3 font-mono">···{invitation.hint}</td>
									<td class="py-2 pr-3 text-dark">{capacityLabel(invitation.max_uses)}</td>
									<td class="py-2 pr-3 text-dark">
										{invitation.uses_count}
										{invitation.max_uses === 0 ? '' : `of ${invitation.max_uses}`}
									</td>
									<td class="py-2 pr-3">
										{#if invitation.revoked}
											<span class="font-medium text-accent-red">revoked</span>
										{:else}
											<span class="font-medium text-accent-green">active</span>
										{/if}
									</td>
									<td class="py-2">
										{#if !invitation.revoked}
											<button
												class="text-sm font-medium text-accent-red underline"
												onclick={() => (revokeTarget = invitation)}
											>
												Revoke
											</button>
										{/if}
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</div>
	</div>
</section>

<ConfirmDialog
	open={revokeTarget !== null}
	title="Revoke this invitation code?"
	description="Players who already redeemed it lose their access, and the code can never be used again."
	confirmLabel="Revoke code"
	confirmColor="red"
	busy={revoking}
	onconfirm={revoke}
	oncancel={() => (revokeTarget = null)}
/>
