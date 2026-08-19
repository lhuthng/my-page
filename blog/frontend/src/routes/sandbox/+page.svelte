<script>
	import SandboxMachine from './SandboxMachine.svelte';
	import { readFloppyFiles, floppyFilesToZip } from '$lib/features/v86/floppy.js';
	import { loadBlankFloppy } from '$lib/players/v86-saves.js';

	let { data } = $props();

	let versionId = $state(data.systems[0]?.id ?? '');
	let zip = $state();
	let busy = $state(false);
	let status = $state('');
	let percent = $state(0);
	let critical = $state(false);
	let machine = $state(null);
	let booted = $state(null);
	let mounted = $state(null);
	let floppyBusy = $state(false);
	let floppyStatus = $state('');
	let floppyCritical = $state(false);
	let floppyIn = $state(false);

	const selected = $derived(data.systems.find((system) => system.id === versionId));

	const formatBytes = (bytes) =>
		bytes < 1048576 ? `${(bytes / 1024).toFixed(0)} KB` : `${(bytes / 1048576).toFixed(1)} MB`;

	const boot = () => {
		if (!selected) return;
		machine = null;
		mounted = null;
		status = '';
		critical = false;
		floppyIn = false;
		floppyStatus = '';
		floppyCritical = false;
		booted = { ...selected };
	};

	const buildIso = () =>
		new Promise((resolve, reject) => {
			const worker = new Worker(new URL('$lib/players/sandbox-disk-worker.js', import.meta.url), {
				type: 'module'
			});
			const finish = (fn, value) => {
				worker.terminate();
				fn(value);
			};
			worker.onmessage = ({ data: message }) => {
				if (message.type === 'progress') {
					status = message.message;
					percent = message.percent ?? 0;
				} else if (message.type === 'done') finish(resolve, message);
				else finish(reject, new Error(message.message));
			};
			worker.onerror = () => finish(reject, new Error('Could not read that file. Is it a .zip?'));
			worker.postMessage({ file: zip });
		});

	// Works at any point while the machine runs, as many times as you like.
	const mount = async () => {
		if (!zip || !machine || busy) return;
		busy = true;
		critical = false;
		percent = 0;
		status = 'Getting your game ready…';
		try {
			const result = await buildIso();
			// A swap ejects first and waits, so say so rather than sitting on the
			// last build message.
			status = mounted ? 'Changing the disc…' : 'Putting the disc in…';
			await machine.insertDisc(result.image);
			mounted = {
				name: zip.name,
				files: result.files,
				payload: result.bytes,
				image: result.image.byteLength
			};
			status = `Ready — your game is in drive D:. Open My Computer to run it.`;
			percent = 100;
		} catch (error) {
			critical = true;
			status = error?.message ?? 'Could not prepare that game.';
		}
		busy = false;
	};

	const eject = async () => {
		if (!machine || busy) return;
		await machine.ejectDisc();
		mounted = null;
		status = 'Disc removed.';
	};

	const insertFloppy = async () => {
		if (!machine || floppyBusy) return;
		floppyBusy = true;
		floppyCritical = false;
		floppyStatus = 'Putting an empty floppy in drive A:…';
		try {
			const floppy = await loadBlankFloppy();
			await machine.insertFloppy(floppy.buffer);
			floppyIn = true;
			floppyStatus = 'Empty floppy inserted in drive A:. Copy files to it, then download them.';
		} catch (error) {
			floppyCritical = true;
			floppyStatus = error?.message ?? 'Could not insert that floppy.';
		}
		floppyBusy = false;
	};

	const ejectFloppy = async () => {
		if (!machine || floppyBusy) return;
		await machine.ejectFloppy();
		floppyIn = false;
		floppyStatus = 'Floppy removed.';
	};

	const downloadFloppy = async () => {
		if (!machine || floppyBusy) return;
		floppyBusy = true;
		floppyCritical = false;
		floppyStatus = 'Reading the floppy…';
		try {
			const image = await machine.getFloppy();
			const files = readFloppyFiles(image);
			if (files.length === 0) throw new Error('The floppy is empty or unreadable.');
			const zipBytes = floppyFilesToZip(files);
			const blob = new Blob([zipBytes], { type: 'application/zip' });
			const url = URL.createObjectURL(blob);
			const link = document.createElement('a');
			link.href = url;
			link.download = `floppy-${new Date().toISOString().slice(0, 10)}.zip`;
			link.click();
			URL.revokeObjectURL(url);
			floppyStatus = `Downloaded ${files.length} file${files.length === 1 ? '' : 's'} from the floppy.`;
		} catch (error) {
			floppyCritical = true;
			floppyStatus = error?.message ?? 'Could not read the floppy.';
		}
		floppyBusy = false;
	};
</script>

<svelte:head>
	<title>v86 Sandbox</title>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<section class="w-full space-y-4">
	<div class="rounded-xl bg-white p-4 drop-shadow-xl">
		<h1 class="text-2xl font-semibold">Sandbox</h1>
		<p class="text-dark/70">
			Start a machine, then put your game in the CD drive. You can change the disc any time without
			restarting. Your files stay on your computer.
		</p>

		{#if data.systems.length === 0}
			<p class="mt-3 text-sm text-red-700">No machine is available right now.</p>
		{:else}
			<div class="mt-4 flex flex-wrap items-end gap-3">
				<label class="grow text-sm sm:max-w-md">
					<span class="mb-1 block font-medium">Machine</span>
					<select
						bind:value={versionId}
						class="w-full rounded-lg border border-dark/15 px-3 py-2"
						disabled={busy}
					>
						{#each data.systems as system (system.id)}
							<option value={system.id}>{system.system_name} v{system.version_number}</option>
						{/each}
					</select>
				</label>
				<button
					class="rounded-lg bg-dark px-4 py-2 text-sm text-white disabled:opacity-50"
					disabled={!selected || busy}
					onclick={boot}
				>
					{booted ? 'Start over' : 'Start'}
				</button>
			</div>
		{/if}
	</div>

	{#if booted}
		<div class="rounded-xl bg-white p-4 drop-shadow-xl">
			<h2 class="mb-2 font-semibold">Floppy drive</h2>
			<div class="flex flex-wrap items-end gap-3">
				<button
					class="rounded-lg bg-dark px-4 py-2 text-sm text-white disabled:opacity-50"
					disabled={!machine || floppyBusy}
					onclick={insertFloppy}
				>
					{floppyIn ? 'New empty floppy' : 'Insert empty floppy'}
				</button>
				{#if floppyIn}
					<button
						class="rounded-lg border border-dark/15 px-4 py-2 text-sm disabled:opacity-50"
						disabled={floppyBusy}
						onclick={ejectFloppy}
					>
						Eject
					</button>
					<button
						class="rounded-lg bg-dark px-4 py-2 text-sm text-white disabled:opacity-50"
						disabled={floppyBusy}
						onclick={downloadFloppy}
					>
						Download contents as .zip
					</button>
				{/if}
			</div>
			{#if floppyStatus}
				<p class="mt-3 truncate text-sm {floppyCritical ? 'text-red-700' : 'text-dark/70'}">
					{floppyStatus}
				</p>
			{/if}
		</div>

		<div class="rounded-xl bg-white p-4 drop-shadow-xl">
			<h2 class="mb-2 font-semibold">CD drive</h2>
			<div class="flex flex-wrap items-end gap-3">
				<label class="grow text-sm sm:max-w-md">
					<span class="mb-1 block font-medium">Your game (.zip)</span>
					<input
						type="file"
						accept=".zip"
						disabled={busy}
						onchange={(event) => (zip = event.currentTarget.files?.[0])}
						class="w-full text-sm"
					/>
				</label>
				<button
					class="rounded-lg bg-dark px-4 py-2 text-sm text-white disabled:opacity-50"
					disabled={!zip || !machine || busy}
					onclick={mount}
				>
					{mounted ? 'Swap disc' : 'Insert disc'}
				</button>
				{#if mounted}
					<button
						class="rounded-lg border border-dark/15 px-4 py-2 text-sm disabled:opacity-50"
						disabled={busy}
						onclick={eject}
					>
						Eject
					</button>
				{/if}
			</div>

			{#if mounted}
				<p class="mt-3 text-sm text-dark/70">
					In the drive: <strong>{mounted.name}</strong>
					· {mounted.files} files · {formatBytes(mounted.image)}
				</p>
			{/if}
			{#if status}
				<p class="mt-3 truncate text-sm {critical ? 'text-red-700' : 'text-dark/70'}">{status}</p>
			{/if}
			{#if busy && percent > 0}
				<div class="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-dark/10">
					<div class="h-full bg-dark transition-all" style="width: {percent}%"></div>
				</div>
			{/if}
		</div>

		{#key booted}
			<SandboxMachine system={booted} onready={(handle) => (machine = handle)} />
		{/key}
	{/if}
</section>
