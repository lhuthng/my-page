<script>
	import { page } from '$app/stores';
	import { onMount } from 'svelte';

	let pending = $state(true);
	let success = $state(false);
	let message = $state('Confirming your subscription...');

	onMount(async () => {
		const token = $page.url.searchParams.get('token');

		if (!token) {
			pending = false;
			success = false;
			message = 'Missing confirmation token.';
			return;
		}

		const res = await fetch(`/api/newsletter/confirm?token=${encodeURIComponent(token)}`);
		const text = await res.text();
		let payload;
		try {
			payload = JSON.parse(text);
		} catch {
			payload = { message: text };
		}

		pending = false;
		success = res.ok;
		message = payload.message ?? (res.ok ? "You're subscribed!" : 'Confirmation failed.');
	});
</script>

<svelte:head>
	<title>Confirm Subscription | Huu Thang's Blog</title>
</svelte:head>

<div class="flex min-h-[calc(100dvh-8rem)] items-center py-4">
	<div class="mx-auto w-full max-w-xl rounded-3xl bg-white p-8 text-center text-dark">
		<h1 class="text-2xl font-bold">
			{#if pending}
				Confirming Subscription
			{:else if success}
				You're Subscribed!
			{:else}
				Confirmation Failed
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
			<div class="text-lg mx-auto mt-6 w-60 duo-btn" data-duo-color="primary">
				<a href="/">Back Home</a>
			</div>
		{/if}
	</div>
</div>
