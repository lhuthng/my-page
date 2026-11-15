<script>
	import { onMount, untrack } from 'svelte';
	import { authorApi } from '$lib/api/author.js';
	import { audiobooks } from '$lib/api/audiobooks.js';
	import Toasts from '$lib/components/ui/Toasts.svelte';

	let { data } = $props();
	// Read once: the id identifies the resource for the lifetime of this page.
	const audiobookId = untrack(() => data.audiobookId);

	// `settings` is the per-book switch. null while loading.
	let settings = $state(null);
	let book = $state(null);
	let keys = $state([]);
	let loading = $state(true);
	let toggling = $state(false);

	// Floating messages, never in flow: a banner here used to push the switch
	// and the key table down the moment something failed.
	let messages = $state([]);
	let nextMessageId = 0;

	function push(tone, text, ttlMs) {
		const id = ++nextMessageId;
		messages = [...messages, { id, tone, text }];
		if (ttlMs) setTimeout(() => dismiss(id), ttlMs);
	}
	function dismiss(id) {
		messages = messages.filter((message) => message.id !== id);
	}
	/** Acknowledged action: disappears on its own. */
	function flash(text) {
		push('success', text, 3500);
	}
	/** Needs a decision or a retry: stays until dismissed. */
	function fail(text) {
		push('error', text);
	}

	let label = $state('');
	let scopes = $state('read,write');
	let ttlHours = $state('');
	let issuing = $state(false);
	let createdKey = $state(null);

	let base = $state('https://your-site.example');
	let copied = $state('');

	const enabled = $derived(settings?.enabled === true);

	onMount(async () => {
		base = window.location.origin;
		try {
			settings = await authorApi.settings(audiobookId);
			await Promise.all([loadBook(), enabled ? reloadKeys() : Promise.resolve()]);
		} catch (e) {
			fail(e?.message ?? 'Could not load the author API settings.');
		} finally {
			loading = false;
		}
	});

	async function loadBook() {
		try {
			const result = await audiobooks.details(audiobookId);
			book = result.audiobook;
		} catch {
			// The title is decorative here; the id is enough to drive the API.
		}
	}

	async function reloadKeys() {
		const result = await authorApi.keys(audiobookId);
		keys = result.keys ?? [];
	}

	async function toggle() {
		const next = !enabled;
		if (
			!next &&
			!confirm(
				'Turning the API off stops every script using this book immediately. Keys are kept and work again if you turn it back on. Continue?'
			)
		) {
			return;
		}
		toggling = true;
		try {
			settings = await authorApi.setEnabled(audiobookId, next);
			if (next) {
				await reloadKeys();
				flash('API enabled for this book.');
			} else {
				keys = [];
				flash('API disabled. Existing keys are paused, not deleted.');
			}
		} catch (e) {
			fail(e?.message ?? 'Could not change the setting.');
		} finally {
			toggling = false;
		}
	}

	async function issue() {
		issuing = true;
		try {
			const payload = { label: label.trim(), scopes };
			const hours = Number(ttlHours);
			if (ttlHours.trim() && Number.isFinite(hours) && hours > 0) {
				payload.ttl_hours = Math.floor(hours);
			}
			createdKey = await authorApi.issueKey(audiobookId, payload);
			label = '';
			ttlHours = '';
			await reloadKeys();
			flash('Key issued. Copy it now — it will not be shown again.');
		} catch (e) {
			fail(e?.message ?? 'Could not issue the key.');
		} finally {
			issuing = false;
		}
	}

	async function revoke(key) {
		if (!confirm(`Revoke "${key.label || key.hint}"? Scripts using it will stop immediately.`)) {
			return;
		}
		try {
			await authorApi.revokeKey(audiobookId, key.id);
			if (createdKey?.id === key.id) createdKey = null;
			await reloadKeys();
			flash('Key revoked.');
		} catch (e) {
			fail(e?.message ?? 'Could not revoke the key.');
		}
	}

	async function copy(text, name) {
		try {
			await navigator.clipboard.writeText(text);
			copied = name;
			flash(`Copied ${name}.`);
			setTimeout(() => {
				if (copied === name) copied = '';
			}, 2000);
		} catch {
			fail('Copy failed — select the text manually.');
		}
	}

	const apiRoot = $derived(`${base}/api/author/audiobooks/id/${audiobookId}`);

	const snippets = $derived({
		manifest: `curl -H "Authorization: Bearer $AUTHOR_KEY" \\
  "${apiRoot}"`,
		diff: `curl -s -H "Authorization: Bearer $AUTHOR_KEY" \\
  "${apiRoot}" > manifest.json

# Every chapter carries sha256 + size. Compare locally, then only PUT
# chapters whose hash differs from the local file.
jq -r '.chapters[] | "\\(.number) \\(.sha256)"' manifest.json`,
		upload: `SHA=$(shasum -a 256 chapter-017.mp3 | cut -d' ' -f1)

curl -X PUT \\
  -H "Authorization: Bearer $AUTHOR_KEY" \\
  -H "Content-Type: audio/mpeg" \\
  -H "X-Content-SHA256: $SHA" \\
  --data-binary @chapter-017.mp3 \\
  "${apiRoot}/chapters/17?title=Chapter%2017&duration_seconds=2413"`,
		download: `# One chapter (X-Content-SHA256 comes back on the response)
curl -OJ -H "Authorization: Bearer $AUTHOR_KEY" \\
  "${apiRoot}/chapters/17/audio"

# Batch zip, capped at 2 GiB. Narrow it with ?numbers=1-50,53
curl -OJ -H "Authorization: Bearer $AUTHOR_KEY" \\
  "${apiRoot}/archive?numbers=1-50"`
	});
</script>

<svelte:head>
	<title>Author API - Dashboard | Huu Thang's Blog</title>
</svelte:head>

<section class="flex flex-col gap-4 pb-8">
	<div class="bg-white rounded-xl p-4 flex flex-col gap-1">
		<a href="/dashboard/audiobooks/id/{audiobookId}" class="text-sm text-dark/50"
			>← Back to audiobook</a
		>
		<h1 class="text-2xl font-semibold">Author API</h1>
		<p class="text-sm text-dark/60">
			Scripting access to {book ? `“${book.title}”` : 'this book'}. Off by default and switched on
			per book — book id <span class="font-mono">{audiobookId}</span>.
		</p>
	</div>

	<!-- Floating, so a failure never shifts the switch or the key table. -->
	<Toasts {messages} ondismiss={dismiss} />

	{#if loading}
		<div class="bg-white rounded-xl p-4 text-sm text-dark/50">Loading…</div>
	{:else if settings}
		<div class="bg-white rounded-xl p-4 flex items-start justify-between gap-4 flex-wrap">
			<div class="flex flex-col gap-1 max-w-xl">
				<h2 class="text-lg font-semibold">API for this book</h2>
				<p class="text-sm text-dark/70">
					{#if enabled}
						Anyone holding a key issued below can read and write this book's chapters. Other books
						are not reachable with the same key.
					{:else}
						Off. Nothing under <code class="font-mono">/author</code> answers for this book, and no
						key can be issued until you switch it on.
					{/if}
				</p>
				{#if settings.key_count > 0}
					<p class="text-xs text-dark/50">
						{settings.key_count} key{settings.key_count === 1 ? '' : 's'} on file,
						{settings.active_key_count} usable right now.
					</p>
				{/if}
			</div>
			<div class="flex items-center gap-3">
				<span class="text-sm font-medium">{enabled ? 'Enabled' : 'Disabled'}</span>
				<button
					type="button"
					role="switch"
					aria-checked={enabled}
					aria-label="Enable the author API for this book"
					disabled={toggling}
					onclick={toggle}
					class="relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition {enabled
						? 'bg-accent-green'
						: 'bg-dark/25'} {toggling ? 'opacity-50' : ''}"
				>
					<span
						class="inline-block h-4 w-4 transform rounded-full bg-white transition {enabled
							? 'translate-x-6'
							: 'translate-x-1'}"
					></span>
				</button>
			</div>
		</div>

		{#if enabled}
			<div class="bg-white rounded-xl p-4 flex flex-col gap-3">
				<div>
					<h2 class="text-lg font-semibold">Secret keys</h2>
					<p class="text-sm text-dark/60">
						A key unlocks <strong>this book only</strong>. Only you, the creator of this book, can
						issue one.
					</p>
				</div>

				<form
					class="flex flex-wrap items-end gap-2"
					onsubmit={(e) => {
						e.preventDefault();
						issue();
					}}
				>
					<label class="flex flex-col gap-1 text-sm">
						<span>Label</span>
						<input
							class="border rounded-lg px-2 py-1 text-sm"
							bind:value={label}
							placeholder="e.g. import script"
							maxlength="100"
						/>
					</label>
					<label class="flex flex-col gap-1 text-sm">
						<span>Access</span>
						<select class="border rounded-lg px-2 py-1 text-sm" bind:value={scopes}>
							<option value="read,write">Read &amp; write</option>
							<option value="read">Read only</option>
						</select>
					</label>
					<label class="flex flex-col gap-1 text-sm">
						<span>Expires in (hours, optional)</span>
						<input
							class="border rounded-lg px-2 py-1 text-sm w-40"
							bind:value={ttlHours}
							placeholder="never"
							inputmode="numeric"
						/>
					</label>
					<div class="w-fit duo-btn" data-duo-color="green">
						<button type="submit" disabled={issuing}>
							{issuing ? 'Issuing…' : 'Issue key'}
						</button>
					</div>
				</form>
				<p class="text-xs text-dark/50">
					Leave expiry empty for a key that lives until revoked. A read-only key can read the
					manifest and download chapters, but cannot change anything.
				</p>

				{#if createdKey}
					<div
						class="bg-accent-green/10 border border-accent-green/40 rounded-lg p-3 flex flex-col gap-2"
					>
						<strong class="text-sm">Copy this key now — it is shown only once.</strong>
						<div class="flex items-center gap-2 flex-wrap">
							<code class="font-mono text-xs break-all select-all">{createdKey.key}</code>
							<div class="w-fit duo-btn" data-duo-color="light">
								<button onclick={() => copy(createdKey.key, 'the secret key')}>Copy</button>
							</div>
						</div>
					</div>
				{/if}

				{#if keys.length === 0}
					<p class="text-sm text-dark/50">No keys yet.</p>
				{:else}
					<div class="overflow-x-auto">
						<table class="text-sm w-full text-left">
							<thead class="text-dark/50 border-b">
								<tr>
									<th class="py-2 pr-3">Label</th>
									<th class="py-2 pr-3">Key</th>
									<th class="py-2 pr-3">Access</th>
									<th class="py-2 pr-3">Expires</th>
									<th class="py-2 pr-3">Last used</th>
									<th class="py-2"></th>
								</tr>
							</thead>
							<tbody>
								{#each keys as key (key.id)}
									<tr class="border-b last:border-0 {key.revoked_at ? 'opacity-50' : ''}">
										<td class="py-2 pr-3">{key.label || '—'}</td>
										<td class="py-2 pr-3 font-mono text-xs">{key.hint}</td>
										<td class="py-2 pr-3">{key.scopes}</td>
										<td class="py-2 pr-3 text-xs">{key.expires_at ?? 'never'}</td>
										<td class="py-2 pr-3 text-xs">{key.last_used_at ?? 'never'}</td>
										<td class="py-2 text-right">
											{#if key.revoked_at}
												<span class="text-xs text-dark/50">revoked</span>
											{:else}
												<div class="w-fit duo-btn" data-duo-color="red">
													<button onclick={() => revoke(key)}>Revoke</button>
												</div>
											{/if}
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</div>

			<div class="bg-white rounded-xl p-4 flex flex-col gap-4">
				<div>
					<h2 class="text-lg font-semibold">Quick guide</h2>
					<p class="text-sm text-dark/60">
						Set <code class="font-mono">AUTHOR_KEY</code> to a secret you issued above. The hash in
						the manifest is the SHA-256 of the file you upload — audio is never transcoded, so it
						matches your local <code class="font-mono">shasum</code> exactly. Read, diff, then upload
						only what changed.
					</p>
				</div>

				{#each [{ title: '1. Read the manifest (metadata + every chapter hash)', text: snippets.manifest }, { title: '2. Diff locally against your files', text: snippets.diff }, { title: '3. Upload one chapter (idempotent)', text: snippets.upload }, { title: '4. Download one chapter or a batch zip', text: snippets.download }] as step (step.title)}
					<div class="flex flex-col gap-1">
						<div class="flex items-center justify-between gap-2">
							<h3 class="text-sm font-semibold">{step.title}</h3>
							<div class="w-fit duo-btn" data-duo-color="light">
								<button onclick={() => copy(step.text, 'the command')}>
									{copied === 'the command' ? 'Copied' : 'Copy'}
								</button>
							</div>
						</div>
						<pre
							class="text-xs bg-dark/5 rounded-lg p-3 overflow-x-auto whitespace-pre-wrap">{step.text}</pre>
					</div>
				{/each}

				<div class="text-sm text-dark/70 flex flex-col gap-1">
					<p>
						<strong>Upload replies:</strong>
						<code class="font-mono">201 created</code>,
						<code class="font-mono">200 replaced</code>, or
						<code class="font-mono">200 unchanged</code> (the slot already holds those exact bytes —
						nothing is written). A mismatched
						<code class="font-mono">X-Content-SHA256</code> is rejected with 422 before anything is
						stored.
					</p>
					<p>
						Chapter numbers are slots you choose, not database ids. A <code class="font-mono"
							>PUT</code
						> to the same number is an upsert — safe to retry.
					</p>
				</div>
			</div>
		{/if}
	{/if}
</section>
