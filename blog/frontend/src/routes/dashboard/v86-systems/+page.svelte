<script>
	import { auth } from '$lib/auth/user.svelte.js';
	import { Sha256 } from '$lib/features/v86/sha256.js';
	import { createZstdCompress, teardownZstd } from '$lib/features/v86/zstd.js';
	import { onMount } from 'svelte';

	let { data } = $props();
	let systems = $state(data.systems);
	let name = $state('');
	let replacingSystemId = $state('');
	let image = $state();
	let busy = $state(false);
	let status = $state('');
	let critical = $state(false);
	let serverOk = $state(false);
	let activeUploads = $state([]);

	const CHUNK_SIZE = 256 * 1024;

	const request = async (url, options = {}) => {
		const response = await fetch(url, {
			...options,
			headers: { Authorization: auth(), ...(options.headers ?? {}) }
		});
		if (!response.ok) throw new Error(await response.text());
		return response;
	};

	const refreshSystems = async () => {
		const res = await fetch('/api/v86/systems', { headers: { Authorization: auth() } });
		if (res.ok) systems = await res.json();
	};

	onMount(async () => {
		try {
			const res = await request('/api/v86/systems/status');
			const data = await res.json();
			serverOk = data.ok === true;
			activeUploads = data.active_uploads ?? [];
			if (activeUploads.length > 0) {
				busy = true;
				critical = false;
				const latest = activeUploads[0];
				status = latest.message ?? 'An upload is in progress…';
			}
		} catch {
			serverOk = false;
		}
	});

	const hashFile = async (file) => {
		const hasher = new Sha256();
		const chunkSize = 1024 * 1024;
		for (let offset = 0; offset < file.size; offset += chunkSize) {
			const slice = file.slice(offset, Math.min(offset + chunkSize, file.size));
			const buf = await slice.arrayBuffer();
			hasher.update(new Uint8Array(buf));
			status = `Hashing… ${Math.round((offset / file.size) * 100)}%`;
		}
		return hasher.digestHex();
	};

	// The base IMG must be a bootable disk image. The server used to check this
	// after upload; it now runs client-side before anything is sent.
	const checkBootSignature = async (file) => {
		const head = new Uint8Array(await file.slice(0, 512).arrayBuffer());
		if (head.length < 512 || head[510] !== 0x55 || head[511] !== 0xaa) {
			throw new Error('The image does not contain a valid boot-sector signature (0x55AA).');
		}
	};

	const upload = async () => {
		if (!image || !name.trim() || busy) return;
		busy = true;
		critical = false;
		let uploadId;
		let interval;
		try {
			await checkBootSignature(image);
			const sha256 = await hashFile(image);
			const existing = systems.find((system) => system.id === Number(replacingSystemId));
			const response = await request('/api/v86/systems/upload', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					system_id: existing?.id ?? null,
					expected_current_version: existing?.current_version ?? 0,
					name: name.trim(),
					platform_key: 'windows9x',
					file_name: image.name,
					size_bytes: image.size,
					sha256
				})
			});
			const session = await response.json();
			uploadId = session.upload_id;

			if (!session.reuse) {
				const zstdCompress = createZstdCompress({ workers: 3 });
				const chunkCount = session.chunk_count;
				let nextPart = 0;
				let uploadError = null;

				const uploadPart = async (partIndex) => {
					const start = partIndex * CHUNK_SIZE;
					const end = Math.min(start + CHUNK_SIZE, image.size);
					const slice = image.slice(start, end);
					const buf = await slice.arrayBuffer();
					const raw = new Uint8Array(buf);
					const padded = new Uint8Array(CHUNK_SIZE);
					padded.set(raw);
					const compressed = await zstdCompress(padded.buffer);
					const res = await fetch(`/api/v86/systems/upload/${uploadId}/part/${partIndex}`, {
						method: 'PUT',
						headers: { Authorization: auth(), 'Content-Type': 'application/octet-stream' },
						body: compressed
					});
					if (!res.ok) throw new Error(await res.text());
					status = `Uploading image… ${Math.round(((partIndex + 1) / chunkCount) * 100)}%`;
				};

				await Promise.all(
					Array.from({ length: 8 }, async () => {
						while (nextPart < chunkCount && !uploadError) {
							const partIndex = nextPart++;
							try {
								await uploadPart(partIndex);
							} catch (e) {
								uploadError = e;
								throw e;
							}
						}
					})
				);
				if (uploadError) throw uploadError;
				teardownZstd();
			}

			console.log('[upload] parts uploaded, calling /complete');
			status = 'Verifying immutable image chunks…';
			interval = setInterval(async () => {
				try {
					const res = await fetch(`/api/v86/systems/upload/${uploadId}`, {
						headers: { Authorization: auth() }
					});
					if (res.ok) {
						const data = await res.json();
						if (data.chunk_progress) {
							status =
								data.chunk_progress.message ||
								`Verifying chunk ${data.chunk_progress.completed_chunks}/${data.chunk_progress.total_chunks}`;
						} else if (data.status === 'consumed') {
							clearInterval(interval);
							busy = false;
							critical = false;
							status = 'System version ready.';
							image = undefined;
							replacingSystemId = '';
							await refreshSystems();
						} else if (data.status === 'failed') {
							clearInterval(interval);
							busy = false;
							critical = true;
							status = data.error_message ?? 'Image preparation failed.';
						}
					}
				} catch {
					/* transient poll failure; keep polling */
				}
			}, 800);
			await request(`/api/v86/systems/upload/${uploadId}/complete`, {
				method: 'POST'
			});
		} catch (error) {
			if (interval) clearInterval(interval);
			if (uploadId) {
				await fetch(`/api/v86/systems/upload/${uploadId}`, {
					method: 'DELETE',
					headers: { Authorization: auth() }
				}).catch(() => {});
			}
			critical = true;
			status = error?.message ?? 'System upload failed.';
			busy = false;
		}
	};

	const updateSystem = async (system, patch) => {
		try {
			await request(`/api/v86/systems/${system.id}`, {
				method: 'PATCH',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					...patch,
					expected_current_version: system.current_version
				})
			});
			if (patch.name !== undefined) system.name = patch.name;
			if (patch.is_active !== undefined) system.is_active = patch.is_active;
			if (patch.is_default === true) {
				systems.forEach((s) => {
					s.is_default = s.id === system.id;
				});
			} else if (patch.is_default === false) {
				system.is_default = false;
			}
		} catch (error) {
			critical = true;
			status = error?.message ?? 'System update failed.';
		}
	};

	const deleteVersion = async (system, version) => {
		if (
			!confirm(
				`Delete ${system.name} v${version.version_number}? This is blocked if a project uses it.`
			)
		)
			return;
		try {
			await request(`/api/v86/systems/${system.id}/versions/${version.id}`, {
				method: 'DELETE'
			});
			await refreshSystems();
		} catch (error) {
			critical = true;
			status = error?.message ?? 'Version deletion failed.';
		}
	};

	const deleteSystem = async (system) => {
		if (!confirm(`Delete ${system.name} and every unreferenced image version?`)) return;
		try {
			await request(`/api/v86/systems/${system.id}`, { method: 'DELETE' });
			await refreshSystems();
			if (replacingSystemId === String(system.id)) replacingSystemId = '';
		} catch (error) {
			critical = true;
			status = error?.message ?? 'System deletion failed.';
		}
	};
</script>

<svelte:head><title>v86 Systems | Dashboard</title></svelte:head>

<section class="space-y-4">
	<div class="rounded-xl bg-white p-4 drop-shadow-xl">
		<h1 class="text-2xl font-semibold">v86 Systems</h1>
		<p class="text-dark/70">
			Base images are immutable. Replacing a system creates a new version; existing projects stay
			pinned.
		</p>
	</div>

	{#if !serverOk}
		<div class="rounded-xl bg-accent-red/10 p-3 text-sm text-accent-red">
			Could not reach the server status endpoint. Upload progress may be unavailable.
		</div>
	{:else if activeUploads.length > 0}
		<div class="rounded-xl bg-amber-50 p-3 text-sm text-amber-700">
			{#each activeUploads as upload}
				<div>
					<span class="font-semibold">Upload in progress:</span>
					{upload.message ?? `${upload.completed_chunks}/${upload.total_chunks}`}
				</div>
			{/each}
		</div>
	{/if}

	<form
		class="grid grid-cols-1 gap-4 rounded-xl bg-white p-4 drop-shadow-xl lg:grid-cols-2"
		onsubmit={(event) => {
			event.preventDefault();
			upload();
		}}
	>
		<label class="flex flex-col gap-1 text-sm font-semibold text-dark">
			System name
			<input
				class="w-full rounded-lg border-2 border-dark/25 bg-white px-3 py-2 font-normal outline-none focus:border-dark"
				bind:value={name}
				required
			/>
		</label>
		<label class="flex flex-col gap-1 text-sm font-semibold text-dark">
			Action
			<select
				class="w-full rounded-lg border-2 border-dark/25 bg-white px-3 py-2 font-normal outline-none focus:border-dark"
				bind:value={replacingSystemId}
			>
				<option value="">Create new system</option>
				{#each systems as system}
					<option value={system.id}>Replace {system.name}</option>
				{/each}
			</select>
		</label>
		<label class="flex flex-col gap-1 text-sm font-semibold text-dark">
			Platform strategy
			<input
				class="w-full rounded-lg border-2 border-dark/25 bg-slate-100 px-3 py-2 font-normal"
				value="Windows 9x"
				readonly
			/>
		</label>
		<label class="flex flex-col gap-1 text-sm font-semibold text-dark">
			Raw IMG (maximum 2 GiB)
			<input
				class="w-full rounded-lg border-2 border-dashed border-dark/30 bg-white px-3 py-2 font-normal"
				type="file"
				accept=".img,application/octet-stream"
				required
				onchange={(event) => (image = event.currentTarget.files?.[0])}
			/>
		</label>
		<div
			class="flex flex-col items-start justify-between gap-3 border-t border-dark/15 pt-3 lg:col-span-2 sm:flex-row sm:items-center"
		>
			<p class:text-accent-red={critical} class="min-h-5 text-sm">{status}</p>
			<button
				class="rounded-lg bg-dark px-4 py-2 font-semibold text-white shadow transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
				disabled={busy}
			>
				{busy ? 'Working…' : 'Upload image'}
			</button>
		</div>
	</form>

	{#each systems as system}
		<article class="rounded-xl bg-white p-4 drop-shadow-xl space-y-3">
			<div class="flex flex-wrap items-center justify-between gap-3">
				<div>
					<h2 class="text-xl font-semibold">
						{system.name}
						{#if system.pending_build && system.versions.length === 0}
							<span class="text-sm font-normal text-amber-600">(building…)</span>
						{/if}
					</h2>
					<p class="text-sm">
						{system.platform_key} · current v{system.current_version} · {system.project_count}
						project(s), {system.published_project_count} published
					</p>
				</div>
				<div class="flex flex-wrap gap-2">
					<button
						class="rounded-lg border-2 border-dark/30 px-3 py-1.5 text-sm font-semibold transition hover:bg-dark hover:text-white"
						onclick={() => {
							const next = prompt('System display name', system.name);
							if (next && next.trim() !== system.name) updateSystem(system, { name: next.trim() });
						}}
					>
						Rename
					</button>
					<button
						class="rounded-lg border-2 border-dark/30 px-3 py-1.5 text-sm font-semibold transition hover:bg-dark hover:text-white"
						onclick={() => updateSystem(system, { is_active: !system.is_active })}
					>
						{system.is_active ? 'Deactivate' : 'Activate'}
					</button>
					<button
						class="rounded-lg border-2 border-dark/30 px-3 py-1.5 text-sm font-semibold transition hover:bg-dark hover:text-white disabled:cursor-not-allowed disabled:opacity-50"
						disabled={system.is_default}
						onclick={() => updateSystem(system, { is_default: true })}
					>
						{system.is_default ? 'Default' : 'Make default'}
					</button>
					<button
						class="rounded-lg border-2 border-accent-red/40 px-3 py-1.5 text-sm font-semibold text-accent-red transition hover:bg-accent-red hover:text-white"
						onclick={() => deleteSystem(system)}
					>
						Delete system
					</button>
				</div>
			</div>
			<ul class="divide-y">
				{#each system.versions as version}
					<li class="flex flex-wrap items-center justify-between gap-2 py-2">
						<span>
							v{version.version_number} · {version.original_file_name} ·
							{(version.size_bytes / 1024 / 1024).toFixed(1)} MiB · {version.chunk_count} chunks
						</span>
						<button
							class="rounded-lg border-2 border-accent-red/40 px-3 py-1.5 text-sm font-semibold text-accent-red transition hover:bg-accent-red hover:text-white disabled:cursor-not-allowed disabled:opacity-50"
							disabled={version.version_number === system.current_version}
							onclick={() => deleteVersion(system, version)}
						>
							Delete
						</button>
					</li>
				{/each}
			</ul>
		</article>
	{/each}
</section>
