<script>
	import { onMount } from 'svelte';
	import { clearV86Cache, formatBytes, measureV86Cache } from '$lib/players/v86-cache.js';

	let stored = $state(null);
	let clearing = $state(false);

	onMount(async () => {
		stored = await measureV86Cache();
	});

	async function clearStored() {
		clearing = true;
		try {
			await clearV86Cache();
			stored = { bytes: 0, count: 0 };
		} finally {
			clearing = false;
		}
	}
</script>

<section>
	<h1>Privacy Policy</h1>

	<p>I collect and use data as follows:</p>

	<h3>Authentication</h3>
	<p>
		I use <code>cookies</code>
		to keep you logged in. These are necessary for account functionality.
	</p>

	<h3>Engagement Tracking</h3>
	<p>
		I use <code>localStorage</code>
		to track which posts you have viewed and liked. This data is stored locally on your device to prevent
		spam and ensure accurate view counts.
	</p>

	<h3>Visitor Statistics</h3>
	<p>
		I collect rough daily visitor statistics by country to better understand where traffic comes
		from. This uses Cloudflare's country header and stores only aggregated country counts, not raw
		IP addresses.
	</p>

	<h3>Stored game data</h3>
	<p>
		Retro games here run on an emulator in your browser. To make games start faster on repeat
		visits, pieces of the emulated disk images are cached in your browser's storage and stay on your
		device until you clear them. They never leave your browser except when fetching them from this
		site.
	</p>
	{#if stored}
		{#if stored.count > 0}
			<p>
				Currently kept: <b>{formatBytes(stored.bytes)}</b>
				across {stored.count} files.
				<button type="button" class="clear-btn" disabled={clearing} onclick={clearStored}>
					Clear stored game data
				</button>
			</p>
		{:else}
			<p>Nothing is currently kept.</p>
		{/if}
	{/if}

	<h3>Data Security</h3>
	<ul>
		<li>I do not sell or share your data with third parties.</li>
		<li>Passwords are salted and hashed in my database; I cannot see your actual password.</li>
	</ul>

	<h3>Data Deletion</h3>
	<p>
		If you want your account or data removed, contact me at
		<a class="text-accent-blue" href="mailto:huuthang.l@outlook.com">huuthang.l@outlook.com</a>
		.
	</p>
</section>

<style lang="postcss">
	@reference "../../app.css";

	section {
		@apply rounded-xl bg-white p-4 text-base;
	}

	h1 {
		@apply text-2xl font-semibold;
	}
	h3 {
		@apply pt-4 text-xl font-semibold;
		counter-increment: h3;

		&::before {
			@apply font-semibold;
			content: counter(h3) '. ';
		}
	}
	code {
		@apply rounded-sm bg-dark/20 px-1 text-base;
	}
	.clear-btn {
		@apply ml-1 rounded-md border border-dark/20 px-2 py-0.5 text-sm text-dark/70 transition-colors hover:border-accent-red hover:text-accent-red disabled:opacity-50;
	}
</style>
