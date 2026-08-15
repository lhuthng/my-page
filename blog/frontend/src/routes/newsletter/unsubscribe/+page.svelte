<script>
	import { page } from '$app/stores';
	import { onMount } from 'svelte';

	let token = $state($page.url.searchParams.get('token'));
	let pending = $state(!!token);
	let message = $state(token ? 'Processing your request...' : '');
	let email = $state('');
	let submitting = $state(false);
	let inputError = $state('');

	onMount(async () => {
		if (!token) return;

		try {
			const res = await fetch(`/api/newsletter/unsubscribe?token=${encodeURIComponent(token)}`);
			const text = await res.text();
			let payload;
			try {
				payload = JSON.parse(text);
			} catch {
				payload = { message: text };
			}
			message = payload.message ?? "You've been unsubscribed.";
		} catch {
			message = 'Something went wrong. Please try again.';
		} finally {
			pending = false;
		}
	});

	async function handleUnsubscribe(e) {
		e.preventDefault();
		inputError = '';
		if (!email.trim()) {
			inputError = 'Please enter your email.';
			return;
		}

		submitting = true;
		try {
			const res = await fetch('/api/newsletter/unsubscribe', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ email: email.trim() })
			});
			const text = await res.text();
			let payload;
			try {
				payload = JSON.parse(text);
			} catch {
				payload = { message: text };
			}
			message = payload.message ?? "You've been unsubscribed.";
		} catch {
			message = 'Something went wrong. Please try again.';
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head>
	<title>Unsubscribe | Huu Thang's Blog</title>
</svelte:head>

<div class="flex min-h-[calc(100dvh-8rem)] items-center py-4">
	<div class="mx-auto w-full max-w-xl rounded-3xl bg-white p-8 text-center text-dark">
		<h1 class="text-2xl font-bold">
			{#if token && pending}
				Unsubscribing
			{:else}
				Newsletter
			{/if}
		</h1>

		{#if token}
			<p class="mt-4 text-lg">
				{#if pending}
					Processing your request...
				{:else}
					*{message}
				{/if}
			</p>
			{#if !pending}
				<div class="text-lg mx-auto mt-6 w-60 duo-btn" data-duo-color="primary">
					<a href="/">Back Home</a>
				</div>
			{/if}
		{:else if message}
			<p class="mt-4 text-lg text-accent-green">*{message}</p>
			<div class="text-lg mx-auto mt-6 w-60 duo-btn" data-duo-color="primary">
				<a href="/">Back Home</a>
			</div>
		{:else}
			<p class="mt-4 text-lg">
				Enter the email you subscribed with to stop receiving our newsletter. (´･ω･`)
			</p>
			<form class="mx-auto mt-6 max-w-md" onsubmit={handleUnsubscribe}>
				<input
					type="email"
					bind:value={email}
					placeholder="you@example.com"
					disabled={submitting}
					class="w-full rounded-xl border-2 border-background bg-background/40 px-4 py-3 text-base outline-none disabled:opacity-40"
				/>
				{#if inputError}
					<p class="text-accent-red mt-2 text-sm">*{inputError}</p>
				{/if}
				<div class="duo-btn mx-auto mt-4 w-fit" data-duo-color="primary">
					<button type="submit" disabled={submitting}>
						{submitting ? 'Unsubscribing...' : 'Unsubscribe'}
					</button>
				</div>
			</form>
		{/if}
	</div>
</div>
