<script>
	import { dev } from '$app/environment';
	import { env as publicEnv } from '$env/dynamic/public';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { authState, login, register, resendVerification } from '$lib/auth/user.svelte.js';
	import { onMount } from 'svelte';

	import { fly } from 'svelte/transition';

	let { data } = $props();

	let isLogged = $derived(authState.user !== undefined);

	let isLogging = $state(!data.register);
	let username = $state('');
	let password = $state('');
	let repassword = $state('');
	let email = $state('');
	let pending = $state(false);
	let status = $state(true);
	let message = $state('');
	let redirecting = $state(false);
	let verificationNeeded = $state(false);
	let verificationIdentifier = $state('');
	let resendPending = $state(false);
	let resendStatus = $state(true);
	let resendMessage = $state('');
	let turnstileHost = $state();
	let turnstileToken = $state('');
	let turnstileWidgetId;
	const turnstileSiteKey = publicEnv.PUBLIC_TURNSTILE_SITE_KEY ?? '';
	const captchaRequired = !dev;

	let redirectTarget = $derived.by(() => {
		const target = data.redirectTo;
		if (typeof target === 'string' && target.startsWith('/') && !target.startsWith('//'))
			return target;

		return authState.user?.role === 'admin' || authState.user?.role === 'moderator'
			? '/dashboard'
			: '/';
	});

	$effect(() => {
		isLogging = !data.register;
	});

	async function handleLogin(e) {
		e.preventDefault();
		message = '';
		resendMessage = '';
		verificationNeeded = false;
		pending = true;
		const res = await login(username, password);
		pending = false;
		if (!res.status) {
			status = false;
			message = res.message;
			verificationNeeded = res.message.toLowerCase().includes('email not verified');
			if (verificationNeeded) {
				verificationIdentifier = username.trim();
			}
		} else {
			status = true;
			message = '';
			redirecting = true;
			await goto(redirectTarget, { replaceState: true });
		}
	}

	async function handleRegister(e) {
		e.preventDefault();
		if (password !== repassword) {
			status = false;
			message = 'repassword does not match.';
			return;
		}
		message = '';
		pending = true;
		const res = await register(username, password, email);
		pending = false;
		if (!res.status) {
			status = false;
			message = res.message;
		} else {
			status = true;
			message = res.success.message;
			verificationNeeded = false;
		}
	}

	async function handleResendVerification() {
		if (captchaRequired && !turnstileToken) {
			resendStatus = false;
			resendMessage = 'Please complete the captcha first.';
			return;
		}

		resendPending = true;
		const res = await resendVerification(verificationIdentifier, turnstileToken);
		resendPending = false;
		resendStatus = res.status;
		resendMessage = res.message;
		resetTurnstile();
	}

	function resetTurnstile() {
		if (window.turnstile && turnstileWidgetId !== undefined) {
			window.turnstile.reset(turnstileWidgetId);
		}
		turnstileToken = '';
	}

	let handleSubmit = $derived(isLogging ? handleLogin : handleRegister);

	$effect(() => {
		isLogging;
		message = '';
		verificationNeeded = false;
		verificationIdentifier = '';
	});

	$effect(() => {
		isLogging;
		resendMessage = '';
	});

	$effect(() => {
		if (verificationNeeded) {
			loadTurnstile();
		}
	});

	$effect(() => {
		const currentPath = $page.url.pathname;
		if (authState.user && isLogging && !redirecting && currentPath === '/login') {
			redirecting = true;
			goto(redirectTarget, { replaceState: true });
		}
	});

	async function loadTurnstile() {
		if (!captchaRequired || !turnstileSiteKey || !turnstileHost) return;
		if (!window.turnstile) {
			await new Promise((resolve, reject) => {
				const script = document.createElement('script');
				script.src = 'https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit';
				script.async = true;
				script.defer = true;
				script.onload = resolve;
				script.onerror = reject;
				document.head.appendChild(script);
			});
		}
		if (window.turnstile && turnstileWidgetId === undefined) {
			turnstileWidgetId = window.turnstile.render(turnstileHost, {
				sitekey: turnstileSiteKey,
				theme: 'light',
				callback(token) {
					turnstileToken = token;
				},
				'expired-callback'() {
					turnstileToken = '';
				},
				'error-callback'() {
					turnstileToken = '';
				}
			});
		}
	}

	onMount(() => {
		loadTurnstile();
	});
</script>

<div
	class="flex justify-center not-md:flex-col gap-6 w-full items-center min-h-[calc(100dvh-4rem)] lg:min-h-[calc(100dvh-8rem)] py-4"
>
	<div class="px-8 pb-8 pt-6 w-full sm:w-100 rounded-3xl bg-white">
		<form
			class="flex flex-col gap-4 w-full *:items-center text-xl"
			onsubmit={handleSubmit}
			novalidate
		>
			<h3 class="text-2xl font-bold mx-auto">
				{#if isLogging}
					Log In
				{:else}
					Sign Up
				{/if}
			</h3>
			<div
				class="space-y-2 *:rounded-xl *:border-2 *:border-dark/40 *:has-focus:border-dark *:bg-primary/20 *:has-disabled:opacity-40 text-dark"
			>
				<div class="px-2">
					<input
						class="py-1.5 w-full"
						placeholder="Username"
						autocomplete="username"
						bind:value={username}
						disabled={isLogged && isLogging}
					/>
				</div>
				<div class="relative gap-2 px-2">
					<input
						class="flex-1 py-1.5"
						placeholder="Password"
						type="password"
						bind:value={password}
						disabled={isLogged && isLogging}
					/>
					{#if isLogging}
						<button
							type="button"
							class=" absolute right-2 top-1/2 -translate-y-1/2 text-primary/80 hover:text-dark cursor-pointer"
							disabled={isLogged}
						>
							forgot?
						</button>
					{/if}
				</div>
				{#if !isLogging}
					<div class="flex px-2" in:fly={{ x: -10 }}>
						<input
							class="grow py-1.5"
							placeholder="Re-password"
							type="password"
							bind:value={repassword}
						/>
					</div>
					<div class="flex px-2" in:fly={{ x: 10 }}>
						<input class="grow py-1.5" placeholder="Email" type="email" bind:value={email} />
					</div>
				{/if}
			</div>
			{#if message}
				<div class="w-full">
					<span class="text-right" class:text-accent-red={!status} class:text-accent-green={status}>
						*{message}
					</span>
				</div>
			{/if}
			<div class="w-full duo-btn" data-duo-color="primary">
				<button class="w-full" type="submit" disabled={pending || (isLogging && isLogged)}>
					{#if isLogging}
						Log In
					{:else}
						Sign Up
					{/if}
				</button>
			</div>
			{#if isLogging && verificationNeeded}
				<div
					in:fly
					class="rounded-2xl border-2 border-accent-yellow bg-accent-yellow-light-2 p-3 text-lg text-dark/80"
				>
					<p>
						Your account is waiting for an email check. I sent a fresh link if the old one got lost
						in the mailbox fog.
					</p>
					{#if captchaRequired}
						<div class="mt-3" bind:this={turnstileHost}></div>
					{/if}
					<div class="mt-3 duo-btn" data-duo-color="blue">
						<button
							type="button"
							class="w-full"
							disabled={resendPending || (captchaRequired && !turnstileSiteKey)}
							onclick={handleResendVerification}
						>
							{resendPending ? 'Sending...' : 'Send Fresh Link'}
						</button>
					</div>
					{#if captchaRequired && !turnstileSiteKey}
						<p class="mt-2 text-center text-accent-red">Captcha is not configured yet.</p>
					{/if}
					{#if resendMessage}
						<p
							class="mt-2 text-center"
							class:text-accent-green={resendStatus}
							class:text-accent-red={!resendStatus}
						>
							{resendMessage}
						</p>
					{/if}
				</div>
			{/if}
			{#if isLogging && isLogged}
				<div class="flex flex-col items-center w-full">
					<span class="text-accent-green">
						You're are logged as {authState.user.username}
					</span>
					<span class="text-accent-red">
						Redirecting to {redirectTarget}
					</span>
				</div>
			{/if}
			<div class="separator">
				<span>or</span>
			</div>
			<div class="w-full duo-btn" data-duo-color="primary">
				<button type="button" class="w-full" onclick={() => (isLogging = !isLogging)}>
					{#if isLogging}
						Sign Up
					{:else}
						Log In
					{/if}
				</button>
			</div>
		</form>
	</div>
	<div class="p-8 w-full sm:w-80 rounded-3xl bg-white md:rotate-5 origin-bottom-left">
		<div class="space-y-4">
			<p class="text-lg text-dark/80 text-justify">
				You don't need to log in to read posts! Create a profile for a cool avatar when you comment. <span
					class="text-nowrap"
				>
					𐔌՞ ܸ.ˬ.ܸ՞𐦯
				</span>
			</p>
			{#if !isLogging}
				<p in:fly class="text-lg text-dark/80 text-justify">
					Passwords are hashed using a one-way function and never stored in plain text. After
					signing up, check your inbox for the verification link <span class="text-nowrap">
						ヾ(•̀ ヮ &lt;)و
					</span>
					.
				</p>
			{/if}
		</div>
	</div>
</div>

<style lang="postcss">
	@reference "../../app.css";

	.separator {
		@apply relative flex w-full items-center gap-4 text-dark/20;

		&::before {
			@apply h-0.5 grow bg-dark/20 content-[''];
		}
		&::after {
			@apply h-0.5 grow bg-dark/20 content-[''];
		}
	}
</style>
