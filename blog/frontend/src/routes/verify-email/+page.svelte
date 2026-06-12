<script>
	import { page } from '$app/stores';
	import { onMount } from 'svelte';

	let pending = $state(true);
	let success = $state(false);
	let message = $state('Verifying your email...');

	onMount(async () => {
		const token = $page.url.searchParams.get('token');

		if (!token) {
			pending = false;
			success = false;
			message = 'Missing verification token.';
			return;
		}

		const res = await fetch(`/api/auth/verify-email?token=${encodeURIComponent(token)}`);
		let payload;
		try {
			payload = await res.json();
		} catch {
			payload = { message: await res.text() };
		}

		pending = false;
		success = res.ok;
		message = payload.message ?? (res.ok ? 'Email verified.' : 'Verification failed.');
	});
</script>

<div class="flex min-h-[calc(100dvh-8rem)] items-center py-4">
	<div class="mx-auto w-full max-w-xl rounded-3xl bg-white p-8 text-center text-dark">
		<h1 class="text-2xl font-bold">
			{#if pending}
				Verifying Email
			{:else if success}
				Email Verified
			{:else}
				Verification Failed
			{/if}
		</h1>
		<p
			class="mt-4 text-lg"
			class:text-accent-green={success}
			class:text-accent-red={!success && !pending}
		>
			*{message}
		</p>
		{#if !pending}
			<div class="duo-btn duo-primary text-lg mx-auto mt-6 w-60">
				<a href="/login">{success ? 'Log In' : 'Back To Login'}</a>
			</div>
		{/if}
	</div>
</div>
