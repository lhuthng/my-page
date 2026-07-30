<script>
	import { page } from '$app/stores';
	import { saveLogin } from '$lib/auth/user.svelte.js';
	import Footer from '$lib/components/shell/Footer.svelte';
	import Header from '$lib/components/shell/Header.svelte';
	import NavigationSideBar from '$lib/components/shell/NavigationSideBar.svelte';
	import { onMount } from 'svelte';
	import '../app.css';
	import { el } from '$lib/dom/elements.svelte.js';
	import { fade } from 'svelte/transition';
	import ToTop from '$lib/components/shell/ToTop.svelte';
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

	const ignoreRoutes = ['login', 'verify-email', 'reset-password'];

	$effect(() => {
		win.width = innerWidth.current;
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
	});
</script>

<svelte:head>
	<meta property="og:site_name" content={SITE_NAME} />
	<meta property="og:url" content={canonical} />
	<meta name="twitter:url" content={canonical} />
	<meta name="theme-color" content="#ffffff" />

	{#if noindex}
		<meta name="robots" content="noindex, nofollow, noarchive" />
	{:else}
		<meta name="robots" content="index, follow, max-image-preview:large" />
		<link rel="canonical" href={canonical} />
	{/if}

	<link rel="icon" href={`${SITE_ORIGIN}/favicon.ico`} />
	<link
		rel="alternate"
		type="application/rss+xml"
		title={`${SITE_NAME} RSS feed`}
		href={`${SITE_ORIGIN}/rss.xml`}
	/>
</svelte:head>

<div class="fixed w-dvw h-dvh pointer-events-none z-11" bind:this={mDiv}></div>
<div class="relative flex flex-col min-h-screen z-10">
	<div class="absolute pointer-events-none inset-0 z-50" bind:this={pDiv}></div>
	<Header />
	<main class="grow" bind:this={scrollTarget}>
		<div class="relative flex gap-2 lg:gap-4 w-cap">
			{#if !ignoreRoutes.includes(route)}
				<NavigationSideBar {route} />
			{/if}
			<div class="w-full not-lg:overflow-x-hidden">
				{@render children?.()}
			</div>
		</div>
		<ToTop {scrollTarget} />
	</main>
	<Footer />
</div>

<style lang="postcss">
	@reference "../app.css";

	main {
		@apply flex bg-background pt-32 text-dark not-lg:pt-16;
	}
</style>
