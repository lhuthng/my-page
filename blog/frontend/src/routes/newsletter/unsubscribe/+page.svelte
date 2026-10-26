<script>
	import { page } from '$app/stores';
	import { onMount } from 'svelte';

	let token = $state($page.url.searchParams.get('token'));
	let pending = $state(!!token);
	let message = $state(token ? 'Processing your request...' : '');

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
		{:else}
			<p class="mt-4 text-lg">
				To stop receiving the newsletter, open the unsubscribe link at the bottom of any newsletter
				email. (´･ω･`)
			</p>
			<div class="text-lg mx-auto mt-6 w-60 duo-btn" data-duo-color="primary">
				<a href="/">Back Home</a>
			</div>
		{/if}
	</div>
</div>
