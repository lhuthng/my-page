<script>
	import { onMount } from 'svelte';
	import { auth } from '$lib/auth/user.svelte.js';
	import { V86Player } from '$lib/players/V86Player.svelte.js';

	let { data } = $props();

	const player = new V86Player({ runtime: data.runtime, mode: 'capture' });

	let snapshot = $state(data.snapshot);
	let captured = $state(null); // raw ArrayBuffer from save_state()
	let compressed = $state(null); // { bytes: Uint8Array, sha256, elapsedMs }
	let testing = $state(false);
	let busy = $state(false);
	let status = $state('');
	let critical = $state(false);
	let testVariant = $state(1);

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

	onMount(() => {
		player.mount();
		return () => player.unmount();
	});

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

	const recapture = async () => {
		captured = null;
		compressed = null;
		testing = false;
		critical = false;
		status = 'Rebooting for a fresh capture…';
		await player.rebootWithSnapshot(null, { mode: 'capture' });
		status = '';
	};

	/** Boots the captured state exactly the way the public player will, then
	 *  inserts the launcher CD and save floppy. This is the acceptance check:
	 *  if autorun fires here it will fire for visitors. Nothing is uploaded. */
	const testBoot = async () => {
		if (!captured || busy) return;
		busy = true;
		critical = false;
		testing = true;
		status = 'Restoring the snapshot, then inserting the disc…';
		try {
			player.selectedVariant = testVariant;
			await player.rebootWithSnapshot(captured, { mode: 'play' });
			status = 'Restored. Watch for autorun — the disc is inserted after the guest is up.';
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
			// Deliberately NOT transferred: transferring would detach `captured`
			// and make re-testing or re-uploading impossible without recapturing.
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
						project_id: data.projectId,
						system_version_id: data.runtime.system_version_id,
						game_disk_sha256: data.runtime.game_sha256,
						size_bytes: compressed.bytes.byteLength,
						raw_size_bytes: captured.byteLength,
						sha256: compressed.sha256,
						state_version: 6,
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

			snapshot = await (await request(`/api/v86/projects/id/${data.projectId}/snapshot`)).json();
			status = 'Snapshot published. The demo now restores instead of booting.';
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
		if (!confirm('Delete the published snapshot? The demo will cold-boot again.')) return;
		busy = true;
		try {
			await request(`/api/v86/projects/id/${data.projectId}/snapshot`, { method: 'DELETE' });
			snapshot = { exists: false, stale: false };
			status = 'Snapshot deleted. The demo cold-boots again.';
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
			<a
				href="/dashboard/projects/id/{data.projectId}"
				class="text-sm text-dark/60 hover:underline"
			>
				← Back to project
			</a>
		</div>

		<p class="mt-3 rounded-lg bg-dark/5 p-3 text-sm text-dark/70">
			This machine boots with the base disk and game disk only — <strong>
				no CD and no floppy
			</strong>
			, on purpose. Both drives still exist and Windows still assigns them letters, so inserting media
			later raises a real media-change and autorun fires. Media present at capture time would instead
			be masked when the state is restored.
		</p>
	</div>

	<div class="rounded-xl bg-white p-4 drop-shadow-xl">
		<div class="flex flex-wrap items-center gap-3">
			<span class="text-sm font-medium">Published snapshot:</span>
			{#if !snapshot.exists}
				<span class="rounded-full bg-dark/10 px-3 py-1 text-sm">None — demo cold-boots</span>
			{:else if snapshot.stale}
				<span class="rounded-full bg-amber-100 px-3 py-1 text-sm text-amber-800">
					Stale — recapture (disks changed since capture)
				</span>
			{:else}
				<span class="rounded-full bg-emerald-100 px-3 py-1 text-sm text-emerald-800">
					Ready — {formatBytes(snapshot.size_bytes)}
				</span>
			{/if}
			{#if snapshot.exists}
				<span class="text-sm text-dark/60">
					raw {formatBytes(snapshot.raw_size_bytes)} · captured {snapshot.created_at}
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

			<span class="mx-1 h-6 w-px bg-dark/10"></span>

			<label class="text-sm text-dark/70">
				Test with
				<select
					class="ml-1 rounded-lg border border-dark/15 px-2 py-1 text-sm"
					bind:value={testVariant}
					disabled={busy}
				>
					{#each player.variants as variant (variant.index)}
						<option value={variant.index}>{variant.name || `Variant ${variant.index}`}</option>
					{/each}
				</select>
			</label>
			<button
				class="rounded-lg border border-dark/15 px-3 py-2 text-sm disabled:opacity-50"
				disabled={!captured || busy}
				onclick={testBoot}
			>
				Test boot + insert disc
			</button>
			<button
				class="rounded-lg border border-dark/15 px-3 py-2 text-sm disabled:opacity-50"
				disabled={busy}
				onclick={recapture}
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
		</div>

		{#if captured}
			<p class="text-sm text-dark/70">
				Captured {formatBytes(captured.byteLength)} raw{#if compressed}, compressed to
					{formatBytes(compressed.bytes.byteLength)} ({(
						captured.byteLength / compressed.bytes.byteLength
					).toFixed(1)}× smaller){/if}.
				{#if !testing}
					Test it before publishing — that is the only way to confirm autorun fires.
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
