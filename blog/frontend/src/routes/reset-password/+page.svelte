<script>
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { resetPassword } from '$lib/auth/user.svelte.js';
	import Eye from '$lib/components/svgs/Eye.svelte';
	import { fly } from 'svelte/transition';

	let password = $state('');
	let repassword = $state('');
	let showPassword = $state(false);
	let showRepassword = $state(false);
	let pending = $state(false);
	let status = $state(true);
	let message = $state('');
	let changed = $state(false);
	let token = $derived($page.url.searchParams.get('token') ?? '');

	async function handleSubmit(e) {
		e.preventDefault();
		message = '';

		if (!token) {
			status = false;
			message = 'Reset token is missing.';
			return;
		}

		if (password !== repassword) {
			status = false;
			message = 'repassword does not match.';
			return;
		}

		pending = true;
		const res = await resetPassword(token, password);
		pending = false;
		status = res.status;
		message = res.message;
		changed = res.status;
	}
</script>

<div
	class="flex min-h-[calc(100dvh-4rem)] w-full items-center justify-center py-4 lg:min-h-[calc(100dvh-8rem)]"
>
	<div class="w-full rounded-3xl bg-white px-8 pb-8 pt-6 sm:w-100">
		<form
			class="flex w-full flex-col gap-4 text-xl *:items-center"
			onsubmit={handleSubmit}
			novalidate
		>
			<h3 class="mx-auto text-2xl font-bold">Reset Password</h3>
			<p class="text-center text-lg text-dark/70">
				Set a new password for your account. The reset link only works once.
			</p>
			<div
				class="space-y-2 *:rounded-xl *:border-2 *:border-dark/40 *:has-focus:border-dark *:bg-primary/20 *:has-disabled:opacity-40 text-dark"
			>
				<div class="relative px-2">
					<input
						class="w-full py-1.5 pr-10"
						placeholder="New password"
						type={showPassword ? 'text' : 'password'}
						autocomplete="new-password"
						bind:value={password}
						disabled={changed}
					/>
					<button
						type="button"
						class="absolute right-2 top-1/2 -translate-y-1/2 text-primary/80 hover:text-dark disabled:cursor-not-allowed"
						aria-pressed={showPassword}
						disabled={changed}
						onclick={() => (showPassword = !showPassword)}
						tabindex="-1"
					>
						<Eye class="h-5 w-5" slashed={showPassword} />
						<span class="sr-only">{showPassword ? 'Hide password' : 'Show password'}</span>
					</button>
				</div>
				<div class="relative px-2">
					<input
						class="w-full py-1.5 pr-10"
						placeholder="Re-password"
						type={showRepassword ? 'text' : 'password'}
						autocomplete="new-password"
						bind:value={repassword}
						disabled={changed}
					/>
					<button
						type="button"
						class="absolute right-2 top-1/2 -translate-y-1/2 text-primary/80 hover:text-dark disabled:cursor-not-allowed"
						aria-pressed={showRepassword}
						disabled={changed}
						onclick={() => (showRepassword = !showRepassword)}
					>
						<Eye class="h-5 w-5" slashed={showRepassword} />
						<span class="sr-only">
							{showRepassword ? 'Hide repeated password' : 'Show repeated password'}
						</span>
					</button>
				</div>
			</div>
			{#if message}
				<p
					in:fly
					class="text-center"
					class:text-accent-green={status}
					class:text-accent-red={!status}
				>
					*{message}
				</p>
			{/if}
			{#if changed}
				<div class="w-full duo-btn" data-duo-color="primary">
					<button type="button" class="w-full" onclick={() => goto('/login')}>Back To Login</button>
				</div>
			{:else}
				<div class="w-full duo-btn" data-duo-color="primary">
					<button class="w-full" type="submit" disabled={pending || !token}>
						{pending ? 'Changing...' : 'Change Password'}
					</button>
				</div>
			{/if}
		</form>
	</div>
</div>
