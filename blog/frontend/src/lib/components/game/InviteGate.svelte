<script>
	import { invalidateAll } from '$app/navigation';
	import { api } from '$lib/api/client.js';
	import { auth } from '$lib/auth/user.svelte.js';

	/**
	 * The invitation gate an invited-only game shows instead of its player.
	 * A successful redemption sets the signed invite cookie server-side; the
	 * load re-runs against the fresh cookie and the page swaps the gate for
	 * the player. Registered redeemers additionally earn the account-bound
	 * grant, so the same code works on every device they sign in from.
	 */
	let { title, slug } = $props();

	let code = $state('');
	let error = $state('');
	let busy = $state(false);

	async function redeem() {
		if (busy) return;
		error = '';
		busy = true;
		try {
			await api.post(`games/s/${slug}/invitations/redeem`, {
				body: { code },
				auth: Boolean(auth())
			});
			await invalidateAll();
		} catch (e) {
			error = e?.data?.message || e?.message || 'That code did not work.';
		} finally {
			busy = false;
		}
	}
</script>

<div class="absolute inset-0 grid place-items-center p-6">
	<div class="w-full max-w-md space-y-4 rounded-xl border-2 border-dark bg-white p-6">
		<div class="space-y-1">
			<h3 class="text-lg font-semibold">{title} is invite-only</h3>
			<p class="text-dark/70">Enter the invitation code the author gave you to play.</p>
		</div>
		<form
			class="flex flex-col gap-2 sm:flex-row"
			onsubmit={(e) => {
				e.preventDefault();
				redeem();
			}}
		>
			<input
				class="grow rounded-xl border-2 border-dark px-3 py-2 text-dark outline-none transition-colors placeholder:text-dark/30 focus:bg-primary focus:text-white focus:placeholder:text-white/70"
				placeholder="XXXX-XXXX-XXXX-XXXX"
				bind:value={code}
				autocomplete="off"
				spellcheck="false"
				aria-label="Invitation code"
			/>
			<div class="duo-btn" data-duo-color="green">
				<button type="submit" disabled={busy}>{busy ? 'Checking…' : 'Redeem'}</button>
			</div>
		</form>
		{#if error}
			<p class="text-sm font-medium text-accent-red">{error}</p>
		{/if}
		<p class="text-sm text-dark/60">
			Your access stays in this browser. Sign in before redeeming if you want the game attached to
			your account on every device.
		</p>
	</div>
</div>
