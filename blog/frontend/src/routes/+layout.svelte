<script>
	import { page } from '$app/stores';
	import { saveLogin } from '$lib/auth/user.svelte.js';
	import Footer from '$lib/components/shell/Footer.svelte';
	import Header from '$lib/components/shell/Header.svelte';
	import NavigationSideBar from '$lib/components/shell/NavigationSideBar.svelte';
	import DashboardSideBar from '$lib/components/dashboard/DashboardSideBar.svelte';
	import { onMount } from 'svelte';
	import '../app.css';
	import { el } from '$lib/dom/elements.svelte.js';
	import { fade } from 'svelte/transition';
	import ToTop from '$lib/components/shell/ToTop.svelte';
	import MiniAudiobookPlayer from '$lib/components/audio/MiniAudiobookPlayer.svelte';
	import { win } from '$lib/dom/windows.svelte.js';
	import { canonicalUrl, SITE_NAME, SITE_ORIGIN } from '$lib/config/site.js';
	import { innerWidth } from 'svelte/reactivity/window';

	let { data, children } = $props();

	let route = $derived($page.url.pathname.split('/')[1]);
	let noindex = $derived(
		$page.status >= 400 ||
			['api', 'dashboard', 'login', 'privacy', 'reset-password', 'verify-email'].includes(route)
	);
	let canonical = $derived(canonicalUrl($page.url.pathname));
	let pDiv = $state();
	let mDiv = $state();
	let scrollTarget = $state();
	let shell = $state();

	const ignoreRoutes = ['login', 'verify-email', 'reset-password', 'newsletter'];

	$effect(() => {
		win.width = innerWidth.current;
	});

	// Keep the document language in step with the page on client-side
	// navigation; the server sets it for the initial response.
	$effect(() => {
		document.documentElement.lang = $page.data.lang ?? 'en';
	});

	$effect(() => {
		if (data?.accessToken?.token) {
			saveLogin({
				username: data.user?.username,
				displayName: data.user?.displayName,
				token: data.accessToken.token,
				tokenType: data.accessToken.type,
				role: data.user?.role,
				avatarUrl: data.user?.avatarUrl
			});
		}
	});

	onMount(() => {
		el.pbody = pDiv;
		el.mbody = mDiv;

		// The header is fixed and its height depends on what is inside it (the
		// signed-in row is taller than the signed-out one), so pages cannot
		// hard-code an offset. Publish the measured height once and let the
		// content offset and every sticky rail follow that one value.
		const header = shell.querySelector('.site-header');
		const syncHeaderHeight = () => {
			shell.style.setProperty('--header-height', `${header.getBoundingClientRect().height}px`);
		};
		syncHeaderHeight();
		const observer = new ResizeObserver(syncHeaderHeight);
		observer.observe(header);
		return () => observer.disconnect();
	});
</script>

<svelte:head>
	<meta property="og:site_name" content={SITE_NAME} />
	<meta property="og:url" content={canonical} />
	<meta property="og:locale" content="en_US" />
	<meta name="twitter:url" content={canonical} />
	<meta name="theme-color" content="#ffffff" />

	{#if noindex}
		<meta name="robots" content="noindex, nofollow, noarchive" />
	{:else}
		<meta name="robots" content="index, follow, max-image-preview:large" />
		<link rel="canonical" href={canonical} />
	{/if}

	<link rel="icon" href={`${SITE_ORIGIN}/favicon.ico`} />
	<link rel="icon" type="image/svg+xml" href={`${SITE_ORIGIN}/logo.svg`} />
	<link rel="apple-touch-icon" href={`${SITE_ORIGIN}/logo.png`} />
	<link rel="manifest" href={`${SITE_ORIGIN}/manifest.webmanifest`} />
	<meta name="mobile-web-app-capable" content="yes" />
	<meta name="apple-mobile-web-app-capable" content="yes" />
	<meta name="apple-mobile-web-app-title" content={SITE_NAME} />
	<link
		rel="alternate"
		type="application/rss+xml"
		title={`${SITE_NAME} RSS feed`}
		href={`${SITE_ORIGIN}/rss.xml`}
	/>
</svelte:head>

<div class="fixed w-dvw h-dvh pointer-events-none z-11" bind:this={mDiv}></div>
<div class="shell relative flex flex-col min-h-screen z-10" bind:this={shell}>
	<div class="absolute pointer-events-none inset-0 z-50" bind:this={pDiv}></div>
	<a class="skip-link" href="#main-content">Skip to content</a>
	<Header />
	<main id="main-content" tabindex="-1" class="grow outline-none" bind:this={scrollTarget}>
		<div class="relative flex gap-2 lg:gap-4 w-cap">
			{#if route === 'dashboard'}
				<!-- The +layout.server.js guard already limits /dashboard to mods+ -->
				<DashboardSideBar />
			{:else if !ignoreRoutes.includes(route)}
				<NavigationSideBar {route} />
			{/if}
			<div class="w-full not-lg:overflow-x-hidden">
				{@render children?.()}
			</div>
		</div>
		<ToTop {scrollTarget} />
		<!-- Keeps a playing audiobook reachable on every page. -->
		<MiniAudiobookPlayer />
	</main>
	<Footer />
</div>

<style lang="postcss">
	@reference "../app.css";

	/* One header offset for the whole site: the measured header height plus the
	   page gap. The gap matches the rail spacing (0.5rem) below lg and widens
	   with the rails at lg. The fallbacks match the rendered header before the
	   observer has measured it. */
	.shell {
		--header-height: 4rem;
		--page-gap: 0.5rem;
		--content-top: calc(var(--header-height) + var(--page-gap));
	}

	@media (width >= 64rem) {
		.shell {
			--header-height: 7.5rem;
			--page-gap: 1rem;
		}
	}

	main {
		@apply flex bg-background text-dark;
		padding-top: var(--content-top);
	}

	.skip-link {
		@apply fixed top-2 left-2 z-200 -translate-y-20 rounded-lg bg-white px-4 py-2 font-semibold text-dark shadow-lg transition-transform;
	}

	.skip-link:focus {
		@apply translate-y-0;
	}
</style>
