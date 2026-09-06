<script>
	import { api, ApiError } from '$lib/api/client.js';
	import { fly } from 'svelte/transition';

	let keys = $state([]);
	let loading = $state(true);
	let issuing = $state(false);
	let error = $state('');

	// Issue form
	let label = $state('');
	let ttlHours = $state(24);

	// A freshly issued key, shown exactly once
	let createdKey = $state(null);
	let copied = $state(false);

	// Revoke confirmation
	let keyToRevoke = $state(null);
	let revoking = $state(false);

	const TTL_OPTIONS = [
		{ value: 1, label: '1 hour' },
		{ value: 24, label: '24 hours' },
		{ value: 168, label: '7 days' }
	];

	async function loadKeys() {
		loading = true;
		error = '';
		try {
			keys = await api.get('dashboard/sync-keys');
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to load sync keys.';
		} finally {
			loading = false;
		}
	}

	async function issueKey() {
		issuing = true;
		error = '';
		try {
			createdKey = await api.post('dashboard/sync-keys', {
				body: { label: label.trim(), ttl_hours: ttlHours }
			});
			copied = false;
			label = '';
			await loadKeys();
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to issue a sync key.';
		} finally {
			issuing = false;
		}
	}

	async function copyKey() {
		try {
			await navigator.clipboard.writeText(createdKey.key);
			copied = true;
		} catch {
			copied = false;
		}
	}

	function downloadKey() {
		const blob = new Blob([createdKey.key], { type: 'text/plain' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = 'sync-key.txt';
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);
		URL.revokeObjectURL(url);
	}

	async function revokeKey() {
		revoking = true;
		error = '';
		try {
			await api.delete(`dashboard/sync-keys/${keyToRevoke.id}`);
			keyToRevoke = null;
			await loadKeys();
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to revoke the key.';
		} finally {
			revoking = false;
		}
	}

	function keyStatus(key) {
		if (key.revoked_at) return { label: 'Revoked', class: 'text-accent-red' };
		if (new Date(key.expires_at) < new Date()) return { label: 'Expired', class: 'text-dark/50' };
		return { label: 'Active', class: 'text-accent-green' };
	}

	function formatDate(value) {
		if (!value) return '—';
		return new Date(value.includes('T') ? value : value.replace(' ', 'T') + 'Z').toLocaleString();
	}

	loadKeys();
</script>

<svelte:head>
	<title>Backup &amp; Sync | Dashboard</title>
</svelte:head>

<div class="bg-white rounded-xl p-4 max-w-2xl">
	<h2 class="text-2xl font-semibold mb-2">Sync Keys</h2>
	<p class="text-dark/60 text-sm mb-6">
		Issue a short-lived key that authorizes pulling this environment's data
		(database, media, demos, game artifacts) into a development machine with
		<code class="bg-dark/5 px-1 rounded">sync-pull</code>. A key grants full read
		access to everything — treat it like a password. Keys are shown once and can
		be revoked at any time.
	</p>

	<div class="grid gap-3 sm:grid-cols-[1fr_auto_auto] sm:items-end mb-2">
		<label class="block">
			<span class="text-sm text-dark/60">Label (optional)</span>
			<input
				class="mt-1 w-full rounded-lg border border-dark/15 px-3 py-2 text-sm"
				placeholder="e.g. dev laptop"
				maxlength="100"
				bind:value={label}
			/>
		</label>
		<label class="block">
			<span class="text-sm text-dark/60">Valid for</span>
			<select class="mt-1 w-full rounded-lg border border-dark/15 px-3 py-2 text-sm" bind:value={ttlHours}>
				{#each TTL_OPTIONS as option}
					<option value={option.value}>{option.label}</option>
				{/each}
			</select>
		</label>
		<div class="w-fit duo-btn" data-duo-color="dark">
			<button onclick={issueKey} disabled={issuing}>
				{issuing ? 'Issuing…' : 'Issue Key'}
			</button>
		</div>
	</div>

	{#if error}
		<p class="mt-2 text-sm text-accent-red">{error}</p>
	{/if}

	<div class="mt-6 overflow-x-auto">
		{#if loading}
			<p class="text-sm text-dark/50">Loading…</p>
		{:else if keys.length === 0}
			<p class="text-sm text-dark/50">No sync keys yet.</p>
		{:else}
			<table class="w-full text-sm">
				<thead>
					<tr class="text-left text-dark/50 border-b border-dark/10">
						<th class="py-2 pr-4 font-medium">Label</th>
						<th class="py-2 pr-4 font-medium">Created</th>
						<th class="py-2 pr-4 font-medium">Expires</th>
						<th class="py-2 pr-4 font-medium">Last used</th>
						<th class="py-2 pr-4 font-medium">Status</th>
						<th class="py-2 font-medium"></th>
					</tr>
				</thead>
				<tbody>
					{#each keys as key (key.id)}
						{@const status = keyStatus(key)}
						<tr class="border-b border-dark/5">
							<td class="py-2 pr-4">{key.label || '—'}</td>
							<td class="py-2 pr-4 text-dark/70">{formatDate(key.created_at)}</td>
							<td class="py-2 pr-4 text-dark/70">{formatDate(key.expires_at)}</td>
							<td class="py-2 pr-4 text-dark/70">{formatDate(key.last_used_at)}</td>
							<td class="py-2 pr-4 {status.class}">{status.label}</td>
							<td class="py-2 text-right">
								{#if !key.revoked_at}
									<button
										class="text-accent-red hover:underline disabled:opacity-50"
										disabled={revoking}
										onclick={() => (keyToRevoke = key)}
									>
										Revoke
									</button>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</div>
</div>

{#if createdKey}
	<div class="fixed inset-0 z-40 flex items-center justify-center bg-dark/45 p-4">
		<div
			class="w-full max-w-md rounded-2xl bg-white p-5 shadow-2xl"
			role="dialog"
			aria-modal="true"
			aria-labelledby="sync-key-title"
		>
			<h3 id="sync-key-title" class="text-lg font-semibold mb-1">Your sync key</h3>
			<p class="text-sm text-dark/60 mb-4">
				Copy or download it now — it is <strong>shown only once</strong> and cannot
				be recovered. It expires {formatDate(createdKey.expires_at)}.
			</p>
			<code
				class="block break-all rounded-lg bg-dark/5 px-3 py-2 text-sm select-all"
				in:fly={{ y: 4 }}
			>
				{createdKey.key}
			</code>
			<div class="mt-4 flex flex-wrap gap-2">
				<div class="w-fit duo-btn" data-duo-color="dark">
					<button onclick={copyKey}>{copied ? 'Copied!' : 'Copy'}</button>
				</div>
				<div class="w-fit duo-btn" data-duo-color="light">
					<button onclick={downloadKey}>Download .txt</button>
				</div>
				<div class="w-fit duo-btn" data-duo-color="light">
					<button onclick={() => (createdKey = null)}>Done</button>
				</div>
			</div>
		</div>
	</div>
{/if}

{#if keyToRevoke}
	<div class="fixed inset-0 z-40 flex items-center justify-center bg-dark/45 p-4">
		<div
			class="w-full max-w-md rounded-2xl bg-white p-5 shadow-2xl"
			role="dialog"
			aria-modal="true"
		>
			<h3 class="text-lg font-semibold mb-1">Revoke sync key?</h3>
			<p class="text-sm text-dark/60 mb-4">
				{keyToRevoke.label || 'This key'}
				will stop working immediately. Machines using it will get an
				authorization error on their next request.
			</p>
			<div class="flex gap-2">
				<div class="w-fit duo-btn" data-duo-color="dark">
					<button onclick={revokeKey} disabled={revoking}>
						{revoking ? 'Revoking…' : 'Revoke'}
					</button>
				</div>
				<div class="w-fit duo-btn" data-duo-color="light">
					<button onclick={() => (keyToRevoke = null)}>Cancel</button>
				</div>
			</div>
		</div>
	</div>
{/if}
