<script>
	import { auth } from '$lib/auth/user.svelte.js';

	let { data } = $props();
	let systems = $state(data.systems);
	let name = $state('');
	let replacingSystemId = $state('');
	let image = $state();
	let busy = $state(false);
	let 	status = $state('');
	let critical = $state(false);

	const request = async (url, options = {}) => {
		const response = await fetch(url, {
			...options,
			headers: { Authorization: auth(), ...(options.headers ?? {}) }
		});
		if (!response.ok) throw new Error(await response.text());
		return response;
	};

	const upload = async () => {
		if (!image || !name.trim() || busy) return;
		busy = true;
		critical = false;
		let uploadId;
		let interval;
		try {
			const existing = systems.find((system) => system.id === Number(replacingSystemId));
			console.log('[upload] creating session', { name: name.trim(), replacingSystemId });
			const response = await request('/api/v86/systems/upload', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					system_id: existing?.id ?? null,
					expected_current_version: existing?.current_version ?? 0,
					name: name.trim(),
					platform_key: 'windows95',
					file_name: image.name,
					size_bytes: image.size
				})
			});
			const session = await response.json();
			uploadId = session.upload_id;
			console.log('[upload] session created', uploadId);
			for (
				let index = session.next_chunk_index;
				index * session.chunk_size_bytes < image.size;
				index++
			) {
				const start = index * session.chunk_size_bytes;
				const chunk = image.slice(start, Math.min(start + session.chunk_size_bytes, image.size));
				await request(`/api/v86/systems/upload/${session.upload_id}/chunk/${index}`, {
					method: 'PUT',
					headers: { 'Content-Type': 'application/octet-stream' },
					body: chunk
				});
				status = `Uploading image… ${Math.round(((start + chunk.size) / image.size) * 100)}%`;
			}
			console.log('[upload] all chunks uploaded, calling /complete');
			status = 'Preparing immutable image chunks…';
			interval = setInterval(async () => {
				try {
					const res = await fetch(`/api/v86/systems/upload/${session.upload_id}`, {
						headers: { Authorization: auth() }
					});
					if (res.ok) {
						const data = await res.json();
						console.log('[upload] poll', { status: data.status, progress: data.chunk_progress });
						if (data.chunk_progress) {
							status = data.chunk_progress.message || `Compressing chunk ${data.chunk_progress.completed_chunks}/${data.chunk_progress.total_chunks}`;
						} else if (data.status === 'consumed') {
							console.log('[upload] done — system ready');
							clearInterval(interval);
							busy = false;
							status = 'System version ready.';
							image = undefined;
							const res = await fetch('/api/v86/systems', { headers: { Authorization: auth() } });
							if (res.ok) systems = await res.json();
						} else if (data.status === 'failed') {
							console.log('[upload] failed:', data.error_message);
							clearInterval(interval);
							busy = false;
							critical = true;
							status = data.error_message ?? 'Image preparation failed.';
						}
					}
				} catch (e) {
					console.log('[upload] poll error', e);
				}
			}, 800);
			await request(`/api/v86/systems/upload/${session.upload_id}/complete`, {
				method: 'POST'
			});
			console.log('[upload] /complete returned 202, background task running');
		} catch (error) {
			console.log('[upload] error in main flow', error);
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
				systems.forEach(s => { s.is_default = s.id === system.id; });
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
			system.versions = system.versions.filter(v => v.id !== version.id);
		} catch (error) {
			critical = true;
			status = error?.message ?? 'Version deletion failed.';
		}
	};

	const deleteSystem = async (system) => {
		if (!confirm(`Delete ${system.name} and every unreferenced image version?`)) return;
		try {
			await request(`/api/v86/systems/${system.id}`, { method: 'DELETE' });
			systems = systems.filter(s => s.id !== system.id);
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

	<form
		class="grid grid-cols-1 gap-4 rounded-xl bg-white p-4 drop-shadow-xl lg:grid-cols-2"
		onsubmit={(event) => {
			event.preventDefault();
			upload();
		}}
	>
		<label class="flex flex-col gap-1 text-sm font-semibold text-dark">
			System name
			<input class="w-full rounded-lg border-2 border-dark/25 bg-white px-3 py-2 font-normal outline-none focus:border-dark" bind:value={name} required />
		</label>
		<label class="flex flex-col gap-1 text-sm font-semibold text-dark">
			Action
			<select class="w-full rounded-lg border-2 border-dark/25 bg-white px-3 py-2 font-normal outline-none focus:border-dark" bind:value={replacingSystemId}>
				<option value="">Create new system</option>
				{#each systems as system}
					<option value={system.id}>Replace {system.name}</option>
				{/each}
			</select>
		</label>
		<label class="flex flex-col gap-1 text-sm font-semibold text-dark">
			Platform strategy
			<input class="w-full rounded-lg border-2 border-dark/25 bg-slate-100 px-3 py-2 font-normal" value="Windows 95" readonly />
		</label>
		<label class="flex flex-col gap-1 text-sm font-semibold text-dark">
			Raw IMG (maximum 2 GiB)
			<input class="w-full rounded-lg border-2 border-dashed border-dark/30 bg-white px-3 py-2 font-normal" 
				type="file"
				accept=".img,application/octet-stream"
				required
				onchange={(event) => (image = event.currentTarget.files?.[0])}
			/>
		</label>
		<div class="flex flex-col items-start justify-between gap-3 border-t border-dark/15 pt-3 lg:col-span-2 sm:flex-row sm:items-center">
			<p class:text-accent-red={critical} class="min-h-5 text-sm">{status}</p>
			<button class="rounded-lg bg-dark px-4 py-2 font-semibold text-white shadow transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50" disabled={busy}>
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
					<button class="rounded-lg border-2 border-dark/30 px-3 py-1.5 text-sm font-semibold transition hover:bg-dark hover:text-white" onclick={() => updateSystem(system, { is_active: !system.is_active })}>
						{system.is_active ? 'Deactivate' : 'Activate'}
					</button>
					<button
						class="rounded-lg border-2 border-dark/30 px-3 py-1.5 text-sm font-semibold transition hover:bg-dark hover:text-white disabled:cursor-not-allowed disabled:opacity-50"
						disabled={system.is_default}
						onclick={() => updateSystem(system, { is_default: true })}
					>
						{system.is_default ? 'Default' : 'Make default'}
					</button>
					<button class="rounded-lg border-2 border-accent-red/40 px-3 py-1.5 text-sm font-semibold text-accent-red transition hover:bg-accent-red hover:text-white" onclick={() => deleteSystem(system)}>
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
