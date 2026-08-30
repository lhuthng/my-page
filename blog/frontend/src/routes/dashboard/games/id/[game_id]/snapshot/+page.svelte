<script>
	import { onMount } from 'svelte';
	import { auth } from '$lib/auth/user.svelte.js';
	import { V86Player, V86_TOPOLOGY_VERSION } from '$lib/players/V86Player.svelte.js';

	let { data } = $props();

	const player = new V86Player({ runtime: data.runtime, mode: 'capture' });

	// 0 captures the game-wide machine with an empty drive; a variant index
	// captures with that variant's disc mounted, while the launcher waits out
	// its delay and before it has touched A:.
	let target = $state(0);
	let snapshots = $state(data.snapshots);
	let captured = $state(null); // raw ArrayBuffer from save_state()
	let compressed = $state(null); // { bytes: Uint8Array, sha256, elapsedMs }
	let testing = $state(false);
	let busy = $state(false);
	let status = $state('');
	let critical = $state(false);

	const targets = $derived([
		{ index: 0, label: 'Game-wide (no disc)' },
		...(data.runtime.variants ?? []).map((variant) => ({
			index: variant.index,
			label: variant.name || `Variant ${variant.index}`
		}))
	]);

	const currentVariant = $derived(
		(data.runtime.variants ?? []).find((variant) => variant.index === target)
	);
	const current = $derived(snapshots.find((entry) => entry.variant_index === target));

	const formatBytes = (bytes) => {
		if (bytes == null) return '—';
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / 1048576).toFixed(1)} MB`;
	};

	const request = async (url, options = {}) => {
		const response = await fetch(url, {
			...options,
			headers: { Authorization: auth(), ...(options.headers ?? {}) }
		});
		if (!response.ok) throw new Error(await response.text());
		return response;
	};

	const setError = (message) => {
		critical = true;
		status = message;
		busy = false;
	};

	const refreshStatus = async () => {
		snapshots = await (await request(`/api/v86/games/id/${data.gameId}/snapshot`)).json();
	};

	onMount(() => {
		player.mount();
		return () => player.unmount();
	});

	// Rebuilds the machine with the topology the chosen target needs, since a
	// state can only be restored into the drive layout it was captured on.
	const bootForTarget = async () => {
		captured = null;
		compressed = null;
		testing = false;
		critical = false;
		player.captureVariantIndex = target;
		player.selectedVariant = target > 0 ? target : 1;
		status = target > 0 ? 'Booting with the variant disc…' : 'Booting with an empty drive…';
		await player.rebootWithSnapshot(null, { mode: 'capture' });
		status = '';
	};

	const selectTarget = async (index) => {
		if (busy || index === target) return;
		target = index;
		busy = true;
		await bootForTarget();
		busy = false;
	};

	const capture = async () => {
		if (busy) return;
		busy = true;
		critical = false;
		status = 'Pausing and dumping machine state…';
		try {
			captured = await player.captureState();
			compressed = null;
			status = `Captured ${formatBytes(captured.byteLength)} of raw machine state.`;
		} catch (error) {
			setError(error?.message ?? 'Could not capture the state.');
			return;
		}
		busy = false;
	};

	// Boots the captured state the way the public player will, then attaches the
	// save floppy. If autorun/launcher behaves here it will behave for visitors.
	const testBoot = async () => {
		if (!captured || busy) return;
		busy = true;
		critical = false;
		testing = true;
		status = 'Restoring the snapshot…';
		try {
			await player.rebootWithSnapshot(captured, { mode: 'play', hasCdrom: target > 0 });
			status =
				target > 0
					? 'Restored. The launcher should pick up the save floppy and start the game.'
					: 'Restored. Watch for autorun once the disc is inserted.';
		} catch (error) {
			setError(error?.message ?? 'The test boot failed.');
			return;
		}
		busy = false;
	};

	const compressState = () =>
		new Promise((resolve, reject) => {
			const worker = new Worker('/zstd/zstd-compress-worker.js', { type: 'module' });
			worker.onerror = () => {
				worker.terminate();
				reject(new Error('The zstd worker failed to start.'));
			};
			worker.onmessage = (event) => {
				const message = event.data;
				if (message.type === 'started') {
					status = `Compressing ${formatBytes(message.rawSize)} at zstd level 19…`;
					return;
				}
				if (message.type === 'error') {
					worker.terminate();
					reject(new Error(message.message));
					return;
				}
				if (message.type === 'done') {
					worker.terminate();
					resolve(message);
				}
			};
			// Not transferred: that would detach `captured` and force a recapture
			// before it could be re-tested or re-uploaded.
			worker.postMessage({ buffer: captured, level: 19, wasmUrl: '/zstd/zstd.wasm' });
		});

	const upload = async () => {
		if (!captured || busy) return;
		busy = true;
		critical = false;
		let uploadId;
		try {
			if (!compressed) {
				const result = await compressState();
				const bytes = new Uint8Array(result.compressed);
				const digest = await crypto.subtle.digest('SHA-256', bytes);
				compressed = {
					bytes,
					sha256: [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, '0')).join(''),
					elapsedMs: result.elapsedMs
				};
				status = `Compressed to ${formatBytes(bytes.byteLength)} in ${(
					result.elapsedMs / 1000
				).toFixed(1)}s.`;
			}

			const session = await (
				await request('/api/v86/snapshots/upload', {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						game_id: data.gameId,
						variant_index: target,
						iso_sha256: target > 0 ? currentVariant?.iso_sha256 : null,
						system_version_id: data.runtime.system_version_id,
						game_disk_sha256: data.runtime.game_sha256,
						size_bytes: compressed.bytes.byteLength,
						raw_size_bytes: captured.byteLength,
						sha256: compressed.sha256,
						state_version: 6,
						topology_version: V86_TOPOLOGY_VERSION,
						memory_size: data.runtime.memory_size,
						vga_memory_size: data.runtime.vga_memory_size
					})
				})
			).json();
			uploadId = session.upload_id;

			const total = compressed.bytes.byteLength;
			for (
				let index = session.next_chunk_index;
				index * session.chunk_size_bytes < total;
				index++
			) {
				const start = index * session.chunk_size_bytes;
				const chunk = compressed.bytes.subarray(
					start,
					Math.min(start + session.chunk_size_bytes, total)
				);
				await request(`/api/v86/snapshots/upload/${uploadId}/chunk/${index}`, {
					method: 'PUT',
					headers: { 'Content-Type': 'application/octet-stream' },
					body: chunk
				});
				status = `Uploading snapshot… ${Math.round(((start + chunk.byteLength) / total) * 100)}%`;
			}

			status = 'Verifying and publishing…';
			await request(`/api/v86/snapshots/upload/${uploadId}/complete`, { method: 'POST' });
			await refreshStatus();
			status = 'Snapshot published.';
		} catch (error) {
			if (uploadId) {
				await fetch(`/api/v86/snapshots/upload/${uploadId}`, {
					method: 'DELETE',
					headers: { Authorization: auth() }
				}).catch(() => {});
			}
			setError(error?.message ?? 'The snapshot upload failed.');
			return;
		}
		busy = false;
	};

	const removeSnapshot = async () => {
		if (!confirm('Delete this snapshot?')) return;
		busy = true;
		try {
			await request(`/api/v86/games/id/${data.gameId}/snapshot/${target}`, {
				method: 'DELETE'
			});
			await refreshStatus();
			status = 'Snapshot deleted.';
		} catch (error) {
			setError(error?.message ?? 'Could not delete the snapshot.');
			return;
		}
		busy = false;
	};
</script>

<svelte:head><title>Snapshot Studio | Dashboard</title></svelte:head>

<section class="space-y-4">
	<div class="rounded-xl bg-white p-4 drop-shadow-xl">
		<div class="flex items-start justify-between gap-4">
			<div>
				<h1 class="text-2xl font-semibold">Snapshot Studio</h1>
				<p class="text-dark/70">
					Capture an already-booted machine so visitors skip the {data.runtime.system_name} boot sequence
					entirely.
				</p>
			</div>
			<a href="/dashboard/games/id/{data.gameId}" class="text-sm text-dark/60 hover:underline">
				← Back to game
			</a>
		</div>

		<div class="mt-3 flex flex-wrap items-center gap-2">
			<span class="text-sm font-medium">Capture:</span>
			{#each targets as entry (entry.index)}
				{@const entryStatus = snapshots.find((s) => s.variant_index === entry.index)}
				<button
					class="rounded-full border px-3 py-1 text-sm disabled:opacity-50 {target === entry.index
						? 'border-dark bg-dark text-white'
						: 'border-dark/15'}"
					disabled={busy}
					onclick={() => selectTarget(entry.index)}
				>
					{entry.label}
					{#if entryStatus?.stale}
						<span class="ml-1 text-accent-yellow-dark">• stale</span>
					{:else if entryStatus}
						<span class="ml-1 text-emerald-500">•</span>
					{/if}
				</button>
			{/each}
		</div>

		<p class="mt-3 rounded-lg bg-dark/5 p-3 text-sm text-dark/70">
			{#if target > 0}
				This boots with the <strong>{currentVariant?.name || `variant ${target}`}</strong>
				disc already mounted, so the launcher runs on its own. Capture while it is waiting out its delay
				— it copies saves off A: only after that, which is what lets each visitor's own save still be
				restored.
			{:else}
				This boots with the base and game disk only — <strong>no disc and no floppy</strong>
				, on purpose. Both drives still exist and Windows still assigns them letters, so inserting media
				after the restore raises a real media-change and autorun fires.
			{/if}
			The save floppy is never mounted while capturing, whichever target you pick.
		</p>
	</div>

	<div class="rounded-xl bg-white p-4 drop-shadow-xl space-y-3">
		<div class="flex flex-wrap items-center gap-2">
			<button
				class="rounded-lg bg-dark px-3 py-2 text-sm text-white disabled:opacity-50"
				disabled={!player.running || busy}
				onclick={() => (player.paused ? player.resume() : player.pause())}
			>
				{player.paused ? 'Resume' : 'Pause'}
			</button>
			<button
				class="rounded-lg bg-dark px-3 py-2 text-sm text-white disabled:opacity-50"
				disabled={!player.running || busy || testing}
				onclick={capture}
			>
				Capture state
			</button>
			<button
				class="rounded-lg border border-dark/15 px-3 py-2 text-sm disabled:opacity-50"
				disabled={!captured || busy}
				onclick={testBoot}
			>
				Test boot
			</button>
			<button
				class="rounded-lg border border-dark/15 px-3 py-2 text-sm disabled:opacity-50"
				disabled={busy}
				onclick={async () => {
					busy = true;
					await bootForTarget();
					busy = false;
				}}
			>
				Restart clean
			</button>

			<span class="mx-1 h-6 w-px bg-dark/10"></span>

			<button
				class="rounded-lg bg-emerald-600 px-3 py-2 text-sm text-white disabled:opacity-50"
				disabled={!captured || busy}
				onclick={upload}
			>
				Compress &amp; publish
			</button>

			{#if current}
				<span class="text-sm text-dark/60">
					published {formatBytes(current.size_bytes)}
					{#if current.stale}<span class="text-accent-yellow-dark">· stale, recapture</span>{/if}
				</span>
				<button
					class="rounded-lg border border-red-200 px-3 py-1 text-sm text-red-700 disabled:opacity-50"
					disabled={busy}
					onclick={removeSnapshot}
				>
					Delete
				</button>
			{/if}
		</div>

		{#if captured}
			<p class="text-sm text-dark/70">
				Captured {formatBytes(captured.byteLength)} raw{#if compressed}, compressed to
					{formatBytes(compressed.bytes.byteLength)} ({(
						captured.byteLength / compressed.bytes.byteLength
					).toFixed(1)}× smaller){/if}.
				{#if !testing}
					Test it before publishing — that is the only way to confirm the launcher still behaves.
				{/if}
			</p>
		{/if}

		{#if status}
			<p class="text-sm {critical ? 'text-red-700' : 'text-dark/70'}">{status}</p>
		{/if}
		{#if player.error}
			<p class="text-sm text-red-700">{player.error}</p>
		{/if}
	</div>

	<div class="rounded-xl bg-black p-2 drop-shadow-xl">
		<div class="flex items-center justify-between px-2 py-1 text-xs text-white/60">
			<span>{player.status || 'Starting…'}{player.paused ? ' (frozen)' : ''}</span>
			<button class="hover:text-white" onclick={() => player.captureMouse()}>
				Click to capture mouse
			</button>
		</div>
		<div bind:this={player.shell}>
			<div
				bind:this={player.screenContainer}
				class="w-full"
				style="height: {data.runtime.display_height ?? '520px'}"
			></div>
		</div>
	</div>
</section>
