<script>
	import { dev } from '$app/environment';
	import { env as publicEnv } from '$env/dynamic/public';

	let email = $state('');
	let pending = $state(false);
	let status = $state(true);
	let message = $state('');
	// The captcha stays hidden until Subscribe is hit with no token in hand,
	// and is torn down as soon as the submission finishes.
	let showTurnstile = $state(false);
	// Set when Subscribe is clicked without a token; consumed by the token
	// callback so the form submits itself once the challenge is solved.
	let awaitingSubmit = $state(false);
	let turnstileHost = $state();
	let turnstileToken = $state('');
	let turnstileWidgetId;
	const turnstileSiteKey = publicEnv.PUBLIC_TURNSTILE_SITE_KEY ?? '';
	const captchaRequired = !dev;

	async function loadTurnstileScript() {
		if (window.turnstile) return;
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

	// Renders the widget once both the flag and the mounted host are ready;
	// `turnstileWidgetId` guards against double-rendering.
	$effect(() => {
		if (!captchaRequired || !showTurnstile || !turnstileHost) return;
		loadTurnstileScript().then(() => {
			if (!window.turnstile || turnstileWidgetId !== undefined) return;
			turnstileWidgetId = window.turnstile.render(turnstileHost, {
				sitekey: turnstileSiteKey,
				theme: 'dark',
				callback(token) {
					turnstileToken = token;
					// The user clicked Subscribe and the challenge is done — fire
					// the request instead of making them click a second time.
					if (awaitingSubmit) {
						awaitingSubmit = false;
						submit();
					}
				},
				'expired-callback'() {
					turnstileToken = '';
				},
				'error-callback'() {
					turnstileToken = '';
				}
			});
		});
	});

	function removeTurnstile() {
		if (turnstileWidgetId !== undefined) {
			window.turnstile?.remove(turnstileWidgetId);
			turnstileWidgetId = undefined;
		}
		turnstileToken = '';
		showTurnstile = false;
	}

	async function submit() {
		message = '';
		pending = true;
		try {
			const res = await fetch('/api/newsletter/subscribe', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ email: email.trim(), turnstileToken })
			});

			const text = await res.text();
			let payload;
			try {
				payload = JSON.parse(text);
			} catch {
				payload = { message: text };
			}

			status = res.ok;
			message =
				payload.message ?? (res.ok ? 'Check your inbox to confirm!' : 'Subscription failed.');
			if (res.ok) email = '';
		} catch {
			status = false;
			message = 'Something went wrong. Please try again.';
		} finally {
			pending = false;
			removeTurnstile();
		}
	}

	async function handleSubmit(e) {
		e.preventDefault();
		message = '';
		awaitingSubmit = false;

		if (!email.trim()) {
			status = false;
			message = 'Please enter an email address.';
			return;
		}

		if (captchaRequired && !turnstileToken) {
			awaitingSubmit = true;
			showTurnstile = true;
			status = false;
			message = 'Complete the captcha below to subscribe.';
			return;
		}

		await submit();
	}
</script>

<div class="w-full space-y-2">
	<h3 class="text-xl">Newsletter</h3>
	<p class="text-white/70">Get new posts in your inbox, no spam.</p>
	<form class="flex not-sm:flex-col gap-2" onsubmit={handleSubmit} novalidate>
		<input
			type="email"
			placeholder="you@example.com"
			autocomplete="email"
			bind:value={email}
			disabled={pending}
			class="flex-1 min-w-0 rounded-xl border-2 border-white/40 bg-transparent px-3 py-1.5 text-white placeholder:text-white/40 focus:border-white focus:outline-none disabled:opacity-40"
		/>
		<div class="duo-btn w-fit mx-auto" data-duo-color="primary">
			<button type="submit" disabled={pending}>
				{pending ? 'Subscribing...' : 'Subscribe'}
			</button>
		</div>
	</form>
	{#if showTurnstile}
		<div bind:this={turnstileHost}></div>
	{/if}
	{#if message}
		<p class:text-accent-green={status} class:text-red-400={!status}>
			*{message}
		</p>
	{/if}
</div>
