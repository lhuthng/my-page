<script>
	import { auth } from '$lib/client/user';

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

<div class="bg-white rounded-xl p-6 max-w-lg">
	<h2 class="text-2xl font-semibold mb-2">Download Backup</h2>
	<p class="text-dark/60 text-sm mb-6">
		Creates a ZIP archive containing the database, all uploaded media, and project demos. This may
		take a moment depending on the size of your site.
	</p>

	<button
		onclick={downloadBackup}
		disabled={downloading}
		class="px-6 py-2.5 bg-dark text-white rounded-lg text-base font-medium
	           hover:bg-dark/80 disabled:opacity-50 disabled:cursor-not-allowed
	           transition-colors"
	>
		{downloading ? 'Creating backup…' : 'Download Full Backup'}
	</button>

	{#if error}
		<p class="mt-4 text-sm text-accent-red">{error}</p>
	{/if}
</div>
