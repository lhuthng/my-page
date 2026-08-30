<script>
	import { auth } from '$lib/auth/user.svelte.js';

	//

	let downloading = $state(false);
	let error = $state('');

	async function downloadBackup() {
		downloading = true;
		error = '';

		try {
			const res = await fetch('/api/dashboard/backup', {
				headers: {
					Authorization: auth()
				}
			});

			if (!res.ok) {
				const text = await res.text();
				throw new Error(text || 'Backup request failed');
			}

			const blob = await res.blob();
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			const now = new Date().toISOString().slice(0, 19).replace(/[:-]/g, '');
			a.download = `blog-backup-${now}.zip`;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);
		} catch (e) {
			error = e.message;
		} finally {
			downloading = false;
		}
	}
</script>

<svelte:head>
	<title>Backup | Dashboard</title>
</svelte:head>

<div class="bg-white rounded-xl p-4 max-w-lg">
	<h2 class="text-2xl font-semibold mb-2">Download Backup</h2>
	<p class="text-dark/60 text-sm mb-6">
		Creates a ZIP archive containing the database, all uploaded media, and project demos. This may
		take a moment depending on the size of your site.
	</p>

	<div class="w-fit duo-btn" data-duo-color="dark">
		<button onclick={downloadBackup} disabled={downloading}>
			{downloading ? 'Creating backup…' : 'Download Full Backup'}
		</button>
	</div>

	{#if error}
		<p class="mt-4 text-sm text-accent-red">{error}</p>
	{/if}
</div>
